use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::config::AppConfig;
use crate::metrics::MetricsCollector;
use crate::routes::completions::AppState;

// ---------------------------------------------------------------------------
// GET /api/v1/stats -- Overall gateway metrics
// ---------------------------------------------------------------------------

pub async fn get_stats(
    metrics: web::Data<MetricsCollector>,
) -> HttpResponse {
    let stats = metrics.get_metrics();
    HttpResponse::Ok().json(stats)
}

// ---------------------------------------------------------------------------
// GET /api/v1/stats/providers -- Per-provider health & metrics
// ---------------------------------------------------------------------------

pub async fn get_provider_stats(
    state: web::Data<AppState>,
    metrics: web::Data<MetricsCollector>,
) -> HttpResponse {
    let healthy_map: Vec<(String, bool)> = state
        .providers
        .iter()
        .map(|p| (p.name().to_string(), p.is_healthy()))
        .collect();

    let stats = metrics.get_provider_metrics(&healthy_map);
    HttpResponse::Ok().json(stats)
}

// ---------------------------------------------------------------------------
// GET /api/v1/stats/tiers -- Tier distribution
// ---------------------------------------------------------------------------

pub async fn get_tier_distribution(
    metrics: web::Data<MetricsCollector>,
) -> HttpResponse {
    let tiers = metrics.get_tier_distribution();
    HttpResponse::Ok().json(tiers)
}

// ---------------------------------------------------------------------------
// GET /api/v1/stats/providers/distribution -- Provider request distribution
// ---------------------------------------------------------------------------

pub async fn get_provider_distribution(
    metrics: web::Data<MetricsCollector>,
) -> HttpResponse {
    let dist = metrics.get_provider_distribution();
    HttpResponse::Ok().json(dist)
}

// ---------------------------------------------------------------------------
// GET /api/v1/stats/hourly -- Hourly aggregated data
// ---------------------------------------------------------------------------

pub async fn get_hourly_data(
    metrics: web::Data<MetricsCollector>,
) -> HttpResponse {
    let data = metrics.get_hourly_data();
    HttpResponse::Ok().json(data)
}

// ---------------------------------------------------------------------------
// GET /api/v1/recent-requests -- Recent request log
// ---------------------------------------------------------------------------

pub async fn get_recent_requests(
    metrics: web::Data<MetricsCollector>,
) -> HttpResponse {
    let recent = metrics.get_recent_requests();
    HttpResponse::Ok().json(recent)
}

// ---------------------------------------------------------------------------
// GET /api/v1/models -- Available models from all providers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ModelInfo {
    id: String,
    provider: String,
    is_available: bool,
}

pub async fn get_models(
    state: web::Data<AppState>,
) -> HttpResponse {
    let models: Vec<ModelInfo> = state
        .providers
        .iter()
        .flat_map(|p| {
            let name = p.name().to_string();
            let healthy = p.is_healthy();
            get_known_models(&name).into_iter().map(move |m| ModelInfo {
                id: m,
                provider: name.clone(),
                is_available: healthy,
            })
        })
        .collect();

    HttpResponse::Ok().json(models)
}

/// Return known model IDs for each provider.
fn get_known_models(provider: &str) -> Vec<String> {
    match provider {
        "ollama" => vec![
            "qwen2.5:0.5b", "qwen2.5:1.5b", "qwen2.5:3b", "qwen2.5:7b",
            "llama3.2:1b", "llama3.2:3b", "llama3.1:8b",
            "phi3:mini", "mistral:7b", "codellama:7b",
        ],
        "openai" => vec![
            "gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo",
            "o1", "o1-mini", "o3-mini",
        ],
        "gemini" => vec![
            "gemini-2.0-flash", "gemini-2.0-flash-lite", "gemini-1.5-pro",
            "gemini-1.5-flash",
        ],
        "xai" => vec![
            "grok-2", "grok-2-mini", "grok-beta",
        ],
        "deepseek" => vec![
            "deepseek-chat", "deepseek-coder", "deepseek-reasoner",
        ],
        "openrouter" => vec![
            "meta-llama/llama-3.1-70b-instruct",
            "anthropic/claude-3.5-sonnet",
        ],
        _ => vec![],
    }.into_iter().map(String::from).collect()
}

// ---------------------------------------------------------------------------
// GET /api/v1/config -- Current gateway configuration (safe subset)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SafeConfig {
    host: String,
    port: u16,
    default_cost_profile: String,
    cache_ttl_seconds: u64,
    max_cache_entries: usize,
    pricing_sync_interval_seconds: u64,
    has_database: bool,
    dashboard_url: String,
    providers: Vec<String>,
}

pub async fn get_config(
    config: web::Data<AppConfig>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let providers: Vec<String> = state.providers.iter().map(|p| p.name().to_string()).collect();

    let safe = SafeConfig {
        host: config.host.clone(),
        port: config.port,
        default_cost_profile: config.default_cost_profile.clone(),
        cache_ttl_seconds: config.cache_ttl_seconds,
        max_cache_entries: config.max_cache_entries,
        pricing_sync_interval_seconds: config.pricing_sync_interval_seconds,
        has_database: config.database_url.is_some(),
        dashboard_url: config.dashboard_url.clone(),
        providers,
    };

    HttpResponse::Ok().json(safe)
}
