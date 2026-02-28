use std::sync::Arc;

use tracing::{info, warn};

use crate::error::AppError;
use crate::providers::LlmProvider;
use crate::routing::scorer::RequestScorer;
use crate::types::routing::{CostProfile, RoutingDecision};
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::registry::ModelRegistry;
use super::selector::ModelSelector;

// ---------------------------------------------------------------------------
// CascadeRouter
// ---------------------------------------------------------------------------

/// Orchestrates the full routing pipeline: score -> select -> try primary ->
/// cascade on failure or low quality.
///
/// The cascade router tries the primary model first, then falls back to each
/// fallback model in order until one succeeds or all have been exhausted.
pub struct CascadeRouter {
    scorer: RequestScorer,
    selector: ModelSelector,
}

impl CascadeRouter {
    /// Create a new cascade router backed by the given model registry.
    pub fn new(registry: Arc<ModelRegistry>) -> Self {
        Self {
            scorer: RequestScorer::new(),
            selector: ModelSelector::new(registry),
        }
    }

    /// Synchronous route: score a request and return the routing decision
    /// without actually calling any provider.
    pub fn decide(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<RoutingDecision, AppError> {
        let scoring = self.scorer.score(request);
        let profile = request
            .x_cost_profile
            .as_deref()
            .map(CostProfile::from_str_or_default)
            .unwrap_or(CostProfile::Auto);
        self.selector.select(&scoring, profile)
    }

    /// Full async route: score, select, call the provider, and cascade on
    /// failure or low-quality responses.
    ///
    /// 1. Score the request to determine its complexity tier.
    /// 2. Select the best model via the [`ModelSelector`].
    /// 3. Try the selected model first.
    /// 4. If the response looks low quality, escalate to the next fallback.
    /// 5. If the primary fails, try fallback models in order.
    /// 6. Return the final response together with the routing decision.
    pub async fn route(
        &self,
        request: &ChatCompletionRequest,
        profile: CostProfile,
        providers: &[Arc<dyn LlmProvider>],
    ) -> Result<(ChatCompletionResponse, RoutingDecision), AppError> {
        // 1. Score the request.
        let scoring = self.scorer.score(request);
        info!(
            tier = %scoring.tier,
            complexity = scoring.complexity_score,
            profile = %profile,
            "Request scored"
        );

        // 2. Select model.
        let decision = self.selector.select(&scoring, profile)?;

        // 3. Try the primary model.
        let primary = &decision.selected_model;
        let provider = Self::find_provider(providers, &primary.provider)?;

        match provider
            .chat_completion(request, &primary.model_id)
            .await
        {
            Ok(response) => {
                // 4. Quality heuristic check.
                if Self::is_low_quality(&response, request) {
                    info!(
                        model = %primary.model_id,
                        "Response quality below threshold, attempting escalation"
                    );

                    for fallback in &decision.fallback_models {
                        if let Ok(fb_provider) =
                            Self::find_provider(providers, &fallback.provider)
                        {
                            match fb_provider
                                .chat_completion(request, &fallback.model_id)
                                .await
                            {
                                Ok(fb_response) => {
                                    info!(
                                        model = %fallback.model_id,
                                        "Escalated to fallback model"
                                    );
                                    return Ok((fb_response, decision));
                                }
                                Err(e) => {
                                    warn!(
                                        model = %fallback.model_id,
                                        error = %e,
                                        "Fallback model failed, trying next"
                                    );
                                }
                            }
                        }
                    }

                    // All fallbacks failed — return original response.
                    Ok((response, decision))
                } else {
                    Ok((response, decision))
                }
            }
            Err(primary_err) => {
                warn!(
                    model = %primary.model_id,
                    error = %primary_err,
                    "Primary model failed, trying fallbacks"
                );

                // 5. Primary failed — try fallbacks.
                for fallback in &decision.fallback_models {
                    if let Ok(fb_provider) =
                        Self::find_provider(providers, &fallback.provider)
                    {
                        match fb_provider
                            .chat_completion(request, &fallback.model_id)
                            .await
                        {
                            Ok(fb_response) => {
                                info!(
                                    model = %fallback.model_id,
                                    "Recovered via fallback model"
                                );
                                return Ok((fb_response, decision));
                            }
                            Err(e) => {
                                warn!(
                                    model = %fallback.model_id,
                                    error = %e,
                                    "Fallback model also failed"
                                );
                            }
                        }
                    }
                }

                Err(AppError::ProviderError(format!(
                    "All models failed. Primary error: {primary_err}"
                )))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Find a provider by name from the provider list.
    fn find_provider(
        providers: &[Arc<dyn LlmProvider>],
        name: &str,
    ) -> Result<Arc<dyn LlmProvider>, AppError> {
        providers
            .iter()
            .find(|p| p.name() == name)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Provider '{name}' not configured")))
    }

    /// Quality heuristic: a response is considered low quality if:
    /// - The completion is very short (< 10 tokens)
    /// - AND the request is not a simple yes/no or short-answer question.
    fn is_low_quality(
        response: &ChatCompletionResponse,
        request: &ChatCompletionRequest,
    ) -> bool {
        let completion_tokens = response.usage.completion_tokens;
        if completion_tokens >= 10 {
            return false;
        }

        // Check actual response text length.
        let response_text: String = response
            .choices
            .iter()
            .filter_map(|c| {
                c.message.content.as_ref().and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .collect();

        if response_text.len() >= 40 {
            return false;
        }

        // Simple questions legitimately have short answers.
        if Self::is_simple_question(request) {
            return false;
        }

        true
    }

    /// Heuristic: does the request look like a simple yes/no or single-value
    /// question?
    fn is_simple_question(request: &ChatCompletionRequest) -> bool {
        let text = Self::extract_last_user_message(request);
        let lower = text.to_lowercase();

        if text.len() < 50 {
            return true;
        }

        let simple_patterns = [
            "is it",
            "are you",
            "can you",
            "yes or no",
            "true or false",
            "what is",
            "who is",
            "when is",
        ];

        simple_patterns.iter().any(|p| lower.contains(p))
    }

    /// Extract the last user message text from the request.
    fn extract_last_user_message(request: &ChatCompletionRequest) -> String {
        for msg in request.messages.iter().rev() {
            if msg.role == "user" {
                if let Some(content) = &msg.content {
                    match content {
                        serde_json::Value::String(s) => return s.clone(),
                        serde_json::Value::Array(arr) => {
                            let mut parts = Vec::new();
                            for part in arr {
                                if let Some(text) =
                                    part.get("text").and_then(|v| v.as_str())
                                {
                                    parts.push(text.to_string());
                                }
                            }
                            return parts.join(" ");
                        }
                        _ => {}
                    }
                }
            }
        }
        String::new()
    }
}

impl std::fmt::Debug for CascadeRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CascadeRouter")
            .field("selector", &self.selector)
            .finish()
    }
}
