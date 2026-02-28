use std::sync::Arc;
use std::time::Instant;

use actix_web::{web, HttpResponse};

use crate::error::AppError;
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
    // 3. Find the first healthy provider
    // -----------------------------------------------------------------
    let provider = state
        .providers
        .iter()
        .find(|p| p.is_healthy())
        .ok_or_else(|| AppError::ProviderError("No healthy providers available".to_string()))?;

    // -----------------------------------------------------------------
    // 4. Forward request to the provider
    // -----------------------------------------------------------------
    let provider_start = Instant::now();
    let mut response = provider.chat_completion(&request, &model_id).await?;
    let latency_ms = provider_start.elapsed().as_millis() as u64;

    // -----------------------------------------------------------------
    // 5. Attach router metadata
    // -----------------------------------------------------------------
    let cost_profile = request
        .x_cost_profile
        .as_deref()
        .unwrap_or("auto")
        .to_string();

    response.x_router_metadata = Some(RouterMetadata {
        provider: provider.name().to_string(),
        tier: scoring.tier.to_string(),
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
