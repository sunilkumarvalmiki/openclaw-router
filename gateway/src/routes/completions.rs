use std::sync::Arc;
use std::time::Instant;

use actix_web::{web, HttpResponse};

use crate::error::AppError;
use crate::metrics::MetricsCollector;
use crate::providers::LlmProvider;
use crate::routing::RequestScorer;
use crate::types::{ChatCompletionRequest, RouterMetadata};

/// Shared application state injected into handlers via `web::Data`.
pub struct AppState {
    pub scorer: RequestScorer,
    pub providers: Vec<Arc<dyn LlmProvider>>,
}

/// `POST /v1/chat/completions` -- OpenAI-compatible proxy endpoint.
///
/// 1. Scores the incoming request using the 15-dimensional engine.
/// 2. Resolves the model (from the request or a default).
/// 3. Finds the first healthy provider.
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
    // 2. Determine the model
    // -----------------------------------------------------------------
    let model_id = request
        .model
        .clone()
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    // -----------------------------------------------------------------
    // 3. Find the right provider for the requested model
    // -----------------------------------------------------------------
    let provider = find_provider_for_model(&state.providers, &model_id)?;
    let provider_name = provider.name().to_string();
    let tier_str = scoring.tier.to_string();

    let cost_profile = request
        .x_cost_profile
        .as_deref()
        .unwrap_or("auto")
        .to_string();

    // -----------------------------------------------------------------
    // 4. Forward request to the provider
    // -----------------------------------------------------------------
    let provider_start = Instant::now();
    let result = provider.chat_completion(&request, &model_id).await;
    let latency_ms = provider_start.elapsed().as_millis() as u64;

    match result {
        Ok(mut response) => {
            // ---------------------------------------------------------
            // 5. Record success metrics
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
            // 6. Attach router metadata
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
            // 5b. Record failure metrics
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

/// Infer the correct provider for a given model name.
///
/// Strategy:
/// 1. If the model contains a `/` prefix (e.g. "provider/model"), match by prefix.
/// 2. Match known model-name prefixes to providers (gemini-*, gpt-*, claude-*, etc.).
/// 3. Fall back to the first healthy provider.
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
    let inferred_provider = if model_id.starts_with("gemini") {
        Some("gemini")
    } else if model_id.starts_with("gpt-") || model_id.starts_with("o1") || model_id.starts_with("o3") || model_id.starts_with("o4") {
        Some("openai")
    } else if model_id.starts_with("claude") {
        Some("openai") // Anthropic models via OpenAI-compatible endpoint
    } else if model_id.starts_with("grok") {
        Some("xai")
    } else if model_id.starts_with("deepseek") {
        Some("deepseek")
    } else if model_id.contains('/') {
        Some("openrouter") // Slash-separated models like "meta-llama/llama-3.1-70b"
    } else {
        None
    };

    if let Some(name) = inferred_provider {
        if let Some(p) = providers.iter().find(|p| p.name() == name && p.is_healthy()) {
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
