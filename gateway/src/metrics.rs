use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::SystemTime;

use serde::Serialize;

/// Maximum number of recent requests to keep in memory.
const MAX_RECENT_REQUESTS: usize = 100;
/// Maximum number of hourly data points to keep (7 days).
const MAX_HOURLY_POINTS: usize = 168;

// ---------------------------------------------------------------------------
// Public response types (serialised to JSON for the dashboard)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DashboardMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub uptime_seconds: u64,
    pub total_tokens: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderMetrics {
    pub name: String,
    pub is_healthy: bool,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentRequest {
    pub id: String,
    pub timestamp: u64,
    pub model: String,
    pub provider: String,
    pub tier: String,
    pub cost_profile: String,
    pub status: String,
    pub latency_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cache_hit: bool,
    pub scoring_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TierCount {
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderDistribution {
    pub name: String,
    pub requests: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HourlyDataPoint {
    pub timestamp: u64,
    pub requests: u64,
    pub errors: u64,
    pub avg_latency_ms: f64,
    pub tokens: u64,
}

// ---------------------------------------------------------------------------
// Internal tracking state (behind locks)
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct ProviderState {
    total_requests: u64,
    successful: u64,
    failed: u64,
    total_latency_ms: u64,
    total_tokens: u64,
}

#[derive(Debug)]
struct HourlyBucket {
    hour_ts: u64, // start of the hour (epoch seconds)
    requests: u64,
    errors: u64,
    total_latency_ms: u64,
    tokens: u64,
}

// ---------------------------------------------------------------------------
// MetricsCollector
// ---------------------------------------------------------------------------

/// In-memory metrics collector for the OpenClaw Gateway.
///
/// Thread-safe: atomics for counters, `RwLock` for complex structures.
pub struct MetricsCollector {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_latency_ms: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    total_tokens: AtomicU64,
    total_prompt_tokens: AtomicU64,
    total_completion_tokens: AtomicU64,
    start_time: SystemTime,
    // Tier counters
    tier_simple: AtomicU64,
    tier_medium: AtomicU64,
    tier_complex: AtomicU64,
    tier_reasoning: AtomicU64,
    // Complex state behind RwLock
    recent_requests: RwLock<VecDeque<RecentRequest>>,
    provider_stats: RwLock<std::collections::HashMap<String, ProviderState>>,
    hourly_data: RwLock<VecDeque<HourlyBucket>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            total_prompt_tokens: AtomicU64::new(0),
            total_completion_tokens: AtomicU64::new(0),
            start_time: SystemTime::now(),
            tier_simple: AtomicU64::new(0),
            tier_medium: AtomicU64::new(0),
            tier_complex: AtomicU64::new(0),
            tier_reasoning: AtomicU64::new(0),
            recent_requests: RwLock::new(VecDeque::with_capacity(MAX_RECENT_REQUESTS)),
            provider_stats: RwLock::new(std::collections::HashMap::new()),
            hourly_data: RwLock::new(VecDeque::with_capacity(MAX_HOURLY_POINTS)),
        }
    }

    /// Record a completed (successful) request.
    pub fn record_success(
        &self,
        model: &str,
        provider: &str,
        tier: &str,
        cost_profile: &str,
        latency_ms: u64,
        prompt_tokens: u32,
        completion_tokens: u32,
        cache_hit: bool,
        scoring_ms: f64,
    ) {
        let total = prompt_tokens as u64 + completion_tokens as u64;
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.total_tokens.fetch_add(total, Ordering::Relaxed);
        self.total_prompt_tokens.fetch_add(prompt_tokens as u64, Ordering::Relaxed);
        self.total_completion_tokens.fetch_add(completion_tokens as u64, Ordering::Relaxed);

        if cache_hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        self.increment_tier(tier);
        self.update_provider(provider, latency_ms, total, true);
        self.push_recent(model, provider, tier, cost_profile, "success", latency_ms, prompt_tokens, completion_tokens, cache_hit, scoring_ms);
        self.update_hourly(false, latency_ms, total);
    }

    /// Record a failed request.
    pub fn record_failure(
        &self,
        model: &str,
        provider: &str,
        tier: &str,
        cost_profile: &str,
        latency_ms: u64,
        scoring_ms: f64,
    ) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);

        self.increment_tier(tier);
        self.update_provider(provider, latency_ms, 0, false);
        self.push_recent(model, provider, tier, cost_profile, "error", latency_ms, 0, 0, false, scoring_ms);
        self.update_hourly(true, latency_ms, 0);
    }

    // -----------------------------------------------------------------------
    // Query methods (called by API handlers)
    // -----------------------------------------------------------------------

    pub fn get_metrics(&self) -> DashboardMetrics {
        let total = self.total_requests.load(Ordering::Relaxed);
        let total_lat = self.total_latency_ms.load(Ordering::Relaxed);
        let uptime = self.start_time
            .elapsed()
            .map(|d| d.as_secs())
            .unwrap_or(0);

        DashboardMetrics {
            total_requests: total,
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            avg_latency_ms: if total > 0 { total_lat as f64 / total as f64 } else { 0.0 },
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            uptime_seconds: uptime,
            total_tokens: self.total_tokens.load(Ordering::Relaxed),
            total_prompt_tokens: self.total_prompt_tokens.load(Ordering::Relaxed),
            total_completion_tokens: self.total_completion_tokens.load(Ordering::Relaxed),
        }
    }

    pub fn get_provider_metrics(&self, healthy_map: &[(String, bool)]) -> Vec<ProviderMetrics> {
        let stats = self.provider_stats.read().unwrap();
        healthy_map.iter().map(|(name, is_healthy)| {
            let ps = stats.get(name);
            let (total, successful, failed, total_lat, tokens) = match ps {
                Some(s) => (s.total_requests, s.successful, s.failed, s.total_latency_ms, s.total_tokens),
                None => (0, 0, 0, 0, 0),
            };
            ProviderMetrics {
                name: name.clone(),
                is_healthy: *is_healthy,
                total_requests: total,
                successful_requests: successful,
                failed_requests: failed,
                avg_latency_ms: if total > 0 { total_lat as f64 / total as f64 } else { 0.0 },
                error_rate: if total > 0 { failed as f64 / total as f64 * 100.0 } else { 0.0 },
                total_tokens: tokens,
            }
        }).collect()
    }

    pub fn get_recent_requests(&self) -> Vec<RecentRequest> {
        self.recent_requests.read().unwrap().iter().rev().cloned().collect()
    }

    pub fn get_tier_distribution(&self) -> Vec<TierCount> {
        vec![
            TierCount { name: "Simple".into(), count: self.tier_simple.load(Ordering::Relaxed) },
            TierCount { name: "Medium".into(), count: self.tier_medium.load(Ordering::Relaxed) },
            TierCount { name: "Complex".into(), count: self.tier_complex.load(Ordering::Relaxed) },
            TierCount { name: "Reasoning".into(), count: self.tier_reasoning.load(Ordering::Relaxed) },
        ]
    }

    pub fn get_provider_distribution(&self) -> Vec<ProviderDistribution> {
        let stats = self.provider_stats.read().unwrap();
        let mut out: Vec<ProviderDistribution> = stats
            .iter()
            .map(|(name, s)| ProviderDistribution {
                name: name.clone(),
                requests: s.total_requests,
            })
            .collect();
        out.sort_by(|a, b| b.requests.cmp(&a.requests));
        out
    }

    pub fn get_hourly_data(&self) -> Vec<HourlyDataPoint> {
        let data = self.hourly_data.read().unwrap();
        data.iter().map(|b| HourlyDataPoint {
            timestamp: b.hour_ts,
            requests: b.requests,
            errors: b.errors,
            avg_latency_ms: if b.requests > 0 { b.total_latency_ms as f64 / b.requests as f64 } else { 0.0 },
            tokens: b.tokens,
        }).collect()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn increment_tier(&self, tier: &str) {
        match tier.to_lowercase().as_str() {
            "simple" => { self.tier_simple.fetch_add(1, Ordering::Relaxed); }
            "medium" => { self.tier_medium.fetch_add(1, Ordering::Relaxed); }
            "complex" => { self.tier_complex.fetch_add(1, Ordering::Relaxed); }
            "reasoning" => { self.tier_reasoning.fetch_add(1, Ordering::Relaxed); }
            _ => {}
        }
    }

    fn update_provider(&self, name: &str, latency_ms: u64, tokens: u64, success: bool) {
        let mut stats = self.provider_stats.write().unwrap();
        let entry = stats.entry(name.to_string()).or_default();
        entry.total_requests += 1;
        entry.total_latency_ms += latency_ms;
        entry.total_tokens += tokens;
        if success {
            entry.successful += 1;
        } else {
            entry.failed += 1;
        }
    }

    fn push_recent(
        &self,
        model: &str,
        provider: &str,
        tier: &str,
        cost_profile: &str,
        status: &str,
        latency_ms: u64,
        prompt_tokens: u32,
        completion_tokens: u32,
        cache_hit: bool,
        scoring_ms: f64,
    ) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let req = RecentRequest {
            id: uuid::Uuid::now_v7().to_string(),
            timestamp: now,
            model: model.to_string(),
            provider: provider.to_string(),
            tier: tier.to_string(),
            cost_profile: cost_profile.to_string(),
            status: status.to_string(),
            latency_ms,
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cache_hit,
            scoring_ms,
        };

        let mut recent = self.recent_requests.write().unwrap();
        if recent.len() >= MAX_RECENT_REQUESTS {
            recent.pop_front();
        }
        recent.push_back(req);
    }

    fn current_hour_ts() -> u64 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now - (now % 3600)
    }

    fn update_hourly(&self, is_error: bool, latency_ms: u64, tokens: u64) {
        let hour = Self::current_hour_ts();
        let mut data = self.hourly_data.write().unwrap();

        if let Some(last) = data.back_mut() {
            if last.hour_ts == hour {
                last.requests += 1;
                if is_error { last.errors += 1; }
                last.total_latency_ms += latency_ms;
                last.tokens += tokens;
                return;
            }
        }

        // New hour bucket
        data.push_back(HourlyBucket {
            hour_ts: hour,
            requests: 1,
            errors: if is_error { 1 } else { 0 },
            total_latency_ms: latency_ms,
            tokens,
        });

        while data.len() > MAX_HOURLY_POINTS {
            data.pop_front();
        }
    }
}

impl std::fmt::Debug for MetricsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsCollector")
            .field("total_requests", &self.total_requests.load(Ordering::Relaxed))
            .finish()
    }
}
