use std::sync::Arc;
use std::time::Instant;

use actix_web::{web, HttpResponse};

use crate::error::AppError;
use crate::metrics::MetricsCollector;
use crate::providers::LlmProvider;
use crate::routing::RequestScorer;
use crate::types::{ChatCompletionRequest, RouterMetadata, Tier};

/// Shared application state injected into handlers via `web::Data`.
pub struct AppState {
    pub scorer: RequestScorer,
    pub providers: Vec<Arc<dyn LlmProvider>>,
}

// -------------------------------------------------------------------------
// Priority
// -------------------------------------------------------------------------

/// Request priority parsed from `x_priority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Priority {
    /// Routine tasks — reminders, health checks, heartbeats, summaries,
    /// notifications. Always routed to free local models (Ollama).
    Low,
    /// Standard routing — the scoring engine decides.
    Normal,
    /// Critical tasks — research, coding, learning, analysis.
    /// Always routed to premium cloud providers.
    High,
}

impl Priority {
    fn from_hint(hint: Option<&str>) -> Self {
        match hint.map(|s| s.to_lowercase()).as_deref() {
            Some("low") => Priority::Low,
            Some("high") => Priority::High,
            _ => Priority::Normal,
        }
    }
}

/// Default local model for low-priority tasks. Qwen3 4B is fast (~40 tok/s)
/// and fits entirely in 4 GB VRAM while supporting tool calling.
const LOCAL_MODEL_FAST: &str = "qwen3:4b";

/// Higher-quality local model for medium-complexity local tasks.
const LOCAL_MODEL_QUALITY: &str = "qwen3:8b";

/// `POST /v1/chat/completions` -- OpenAI-compatible proxy endpoint.
///
/// 1. Scores the incoming request using the 15-dimensional engine.
/// 2. Resolves priority and model (auto-routes low-priority to Ollama).
/// 3. Finds the right provider.
/// 4. Forwards the request and appends router metadata.
pub async fn chat_completions(
    state: web::Data<AppState>,
    metrics: web::Data<MetricsCollector>,
    body: web::Json<ChatCompletionRequest>,
) -> Result<HttpResponse, AppError> {
    let request = body.into_inner();

    // -----------------------------------------------------------------
    // 1. Score the request
    // -----------------------------------------------------------------
    let scoring_start = Instant::now();
    let scoring = state.scorer.score(&request);
    let scoring_ms = scoring_start.elapsed().as_secs_f64() * 1000.0;

    // -----------------------------------------------------------------
    // 2. Determine priority and model
    // -----------------------------------------------------------------
    let priority = Priority::from_hint(request.x_priority.as_deref());

    let (model_id, provider) = resolve_model_and_provider(
        &state.providers,
        &request,
        priority,
        scoring.tier,
    )?;

    let provider_name = provider.name().to_string();
    let tier_str = scoring.tier.to_string();

    let cost_profile = request
        .x_cost_profile
        .as_deref()
        .unwrap_or("auto")
        .to_string();

    tracing::info!(
        model = %model_id,
        provider = %provider_name,
        priority = ?priority,
        tier = %tier_str,
        complexity = scoring.complexity_score,
        "Routing request"
    );

    // -----------------------------------------------------------------
    // 3. Forward request to the provider
    // -----------------------------------------------------------------
    let provider_start = Instant::now();
    let result = provider.chat_completion(&request, &model_id).await;
    let latency_ms = provider_start.elapsed().as_millis() as u64;

    match result {
        Ok(mut response) => {
            // ---------------------------------------------------------
            // 4. Record success metrics
            // ---------------------------------------------------------
            metrics.record_success(
                &model_id,
                &provider_name,
                &tier_str,
                &cost_profile,
                latency_ms,
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                false, // cache_hit
                scoring_ms,
            );

            // ---------------------------------------------------------
            // 5. Attach router metadata
            // ---------------------------------------------------------
            response.x_router_metadata = Some(RouterMetadata {
                provider: provider_name,
                tier: tier_str,
                cost_profile,
                cost_usd: None,
                cost_without_router_usd: None,
                savings_percent: None,
                cache_hit: false,
                latency_ms: Some(latency_ms),
                scoring_ms: Some(scoring_ms),
            });

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            // ---------------------------------------------------------
            // 4b. Record failure metrics
            // ---------------------------------------------------------
            metrics.record_failure(
                &model_id,
                &provider_name,
                &tier_str,
                &cost_profile,
                latency_ms,
                scoring_ms,
            );

            Err(e)
        }
    }
}

// -------------------------------------------------------------------------
// Model + provider resolution with priority awareness
// -------------------------------------------------------------------------

/// Resolve the model and provider based on priority, explicit model hint,
/// and scoring tier.
///
/// Priority overrides:
/// - **Low** → always route to Ollama (qwen3:4b for simple, qwen3:8b for medium).
/// - **High** → skip Ollama, use cloud providers only.
/// - **Normal** → standard inference-based routing.
fn resolve_model_and_provider(
    providers: &[Arc<dyn LlmProvider>],
    request: &ChatCompletionRequest,
    priority: Priority,
    tier: Tier,
) -> Result<(String, Arc<dyn LlmProvider>), AppError> {
    // If the user explicitly specified a model, honour it (priority only
    // adjusts the provider preference, not the model choice).
    let explicit_model = request.model.clone();

    match priority {
        // =============================================================
        // LOW PRIORITY → Force Ollama (free, local)
        // =============================================================
        Priority::Low => {
            if let Some(ollama) = find_ollama(providers) {
                let model = explicit_model.unwrap_or_else(|| {
                    // Pick local model based on complexity
                    match tier {
                        Tier::Simple => LOCAL_MODEL_FAST.to_string(),
                        _ => LOCAL_MODEL_QUALITY.to_string(),
                    }
                });
                return Ok((model, ollama));
            }
            // Ollama not available — fall through to normal routing
            tracing::warn!("Low-priority request but Ollama unavailable, falling back");
            let model = explicit_model.unwrap_or_else(|| "gpt-4o-mini".to_string());
            let provider = find_provider_for_model(providers, &model)?;
            Ok((model, provider))
        }

        // =============================================================
        // HIGH PRIORITY → Cloud providers only (skip Ollama)
        // =============================================================
        Priority::High => {
            let model = explicit_model.unwrap_or_else(|| "gpt-4o-mini".to_string());
            let provider = find_cloud_provider_for_model(providers, &model)?;
            Ok((model, provider))
        }

        // =============================================================
        // NORMAL → Standard routing with auto-detection
        //
        // Simple-tier tasks with no explicit model preference are
        // automatically offloaded to Ollama when available.
        // =============================================================
        Priority::Normal => {
            let model = explicit_model.unwrap_or_else(|| {
                // Auto-select: simple tasks → local, others → cloud
                if tier == Tier::Simple {
                    if has_ollama(providers) {
                        LOCAL_MODEL_FAST.to_string()
                    } else {
                        "gpt-4o-mini".to_string()
                    }
                } else {
                    "gpt-4o-mini".to_string()
                }
            });
            let provider = find_provider_for_model(providers, &model)?;
            Ok((model, provider))
        }
    }
}

/// Find the Ollama provider if it is healthy.
fn find_ollama(providers: &[Arc<dyn LlmProvider>]) -> Option<Arc<dyn LlmProvider>> {
    providers
        .iter()
        .find(|p| p.name() == "ollama" && p.is_healthy())
        .cloned()
}

/// Check if Ollama is available.
fn has_ollama(providers: &[Arc<dyn LlmProvider>]) -> bool {
    providers.iter().any(|p| p.name() == "ollama" && p.is_healthy())
}

/// Find a **cloud** (non-Ollama) provider for the model.
/// Used for high-priority routing that must avoid local inference.
fn find_cloud_provider_for_model(
    providers: &[Arc<dyn LlmProvider>],
    model_id: &str,
) -> Result<Arc<dyn LlmProvider>, AppError> {
    let cloud_providers: Vec<Arc<dyn LlmProvider>> = providers
        .iter()
        .filter(|p| p.name() != "ollama" && p.is_healthy())
        .cloned()
        .collect();

    if cloud_providers.is_empty() {
        return Err(AppError::ProviderError(
            "No cloud providers available for high-priority request".to_string(),
        ));
    }

    find_provider_for_model(&cloud_providers, model_id)
}

/// Infer the correct provider for a given model name.
///
/// Strategy:
/// 1. If the model contains a `/` prefix (e.g. "provider/model"), match by prefix.
/// 2. Match known model-name prefixes to providers (gemini-*, gpt-*, claude-*, etc.).
/// 3. Fall back to Ollama for unrecognised models.
/// 4. Last resort: first healthy provider.
fn find_provider_for_model(
    providers: &[Arc<dyn LlmProvider>],
    model_id: &str,
) -> Result<Arc<dyn LlmProvider>, AppError> {
    // 1. Explicit "provider/model" syntax (e.g. "openrouter/meta-llama/...")
    if let Some(slash_idx) = model_id.find('/') {
        let prefix = &model_id[..slash_idx];
        if let Some(p) = providers.iter().find(|p| p.name() == prefix && p.is_healthy()) {
            return Ok(Arc::clone(p));
        }
    }

    // 2. Infer provider from model name prefixes
    let inferred_providers: Vec<&str> = if model_id.starts_with("gemini") {
        vec!["gemini"]
    } else if model_id.starts_with("gpt-") || model_id.starts_with("o1") || model_id.starts_with("o3") || model_id.starts_with("o4") {
        vec!["openai", "github-copilot"] // try OpenAI first, fall back to GitHub Copilot
    } else if model_id.starts_with("claude") {
        vec!["openai", "github-copilot"]
    } else if model_id.starts_with("grok") {
        vec!["xai"]
    } else if model_id.starts_with("deepseek") {
        vec!["deepseek"]
    } else if model_id.starts_with("qwen") || model_id.starts_with("llama") || model_id.starts_with("mistral:") || model_id.starts_with("phi") {
        vec!["ollama"]
    } else if model_id.contains('/') {
        vec!["openrouter"]
    } else {
        vec![]
    };

    for name in &inferred_providers {
        if let Some(p) = providers.iter().find(|p| p.name() == *name && p.is_healthy()) {
            return Ok(Arc::clone(p));
        }
    }

    // 3. Try Ollama for any unrecognised model (local models like "qwen2.5:0.5b")
    if let Some(p) = providers.iter().find(|p| p.name() == "ollama" && p.is_healthy()) {
        return Ok(Arc::clone(p));
    }

    // 4. Last resort: first healthy provider
    providers
        .iter()
        .find(|p| p.is_healthy())
        .cloned()
        .ok_or_else(|| AppError::ProviderError("No healthy providers available".to_string()))
}
