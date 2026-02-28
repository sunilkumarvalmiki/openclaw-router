use std::sync::Arc;

use reqwest::Client;
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::routing::registry::ModelRegistry;

// ---------------------------------------------------------------------------
// OpenRouter API response types
// ---------------------------------------------------------------------------

/// Top-level response from GET https://openrouter.ai/api/v1/models
#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

/// A single model entry from the OpenRouter API.
#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    /// Model identifier (e.g. "meta-llama/llama-3.1-70b-instruct").
    id: String,
    /// Pricing information.
    pricing: Option<OpenRouterPricing>,
}

/// Pricing for an OpenRouter model.
#[derive(Debug, Deserialize)]
struct OpenRouterPricing {
    /// Cost per token for prompts (input), as a string.
    prompt: Option<String>,
    /// Cost per token for completions (output), as a string.
    completion: Option<String>,
}

// ---------------------------------------------------------------------------
// PricingSync
// ---------------------------------------------------------------------------

/// Background service that periodically fetches pricing from external sources
/// and updates the [`ModelRegistry`].
pub struct PricingSync {
    client: Client,
}

impl PricingSync {
    /// Create a new pricing sync service.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build pricing sync HTTP client");

        Self { client }
    }

    /// Spawn a background tokio task that periodically syncs pricing.
    ///
    /// The task runs indefinitely and logs errors without crashing.
    pub fn start(registry: Arc<ModelRegistry>, interval_secs: u64) {
        let sync = Self::new();

        tokio::spawn(async move {
            info!(
                interval_secs = interval_secs,
                "Starting pricing sync background task"
            );

            loop {
                sync.sync_once(&registry).await;
                tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
            }
        });
    }

    /// Run all sync sources once.
    ///
    /// Errors are logged and do not propagate — the service continues even if
    /// a source is temporarily unreachable.
    pub async fn sync_once(&self, registry: &ModelRegistry) {
        info!("Running pricing sync cycle");
        self.sync_openrouter(registry).await;
        // Future: add sync_openai, sync_gemini, etc.
    }

    /// Fetch the latest model pricing from OpenRouter and update matching
    /// models in the registry.
    pub async fn sync_openrouter(&self, registry: &ModelRegistry) {
        let url = "https://openrouter.ai/api/v1/models";

        let response = match self.client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!(error = %e, "Failed to fetch OpenRouter models");
                return;
            }
        };

        if !response.status().is_success() {
            warn!(
                status = %response.status(),
                "OpenRouter API returned non-success status"
            );
            return;
        }

        let body: OpenRouterModelsResponse = match response.json().await {
            Ok(b) => b,
            Err(e) => {
                error!(error = %e, "Failed to parse OpenRouter models response");
                return;
            }
        };

        let mut updated_count = 0u32;

        for model in &body.data {
            if let Some(pricing) = &model.pricing {
                // OpenRouter prices are per-token strings.
                // Convert to per-million-token f64 values.
                let input_per_million = match Self::parse_per_token_to_per_million(
                    pricing.prompt.as_deref(),
                ) {
                    Some(v) => v,
                    None => continue,
                };

                let output_per_million = match Self::parse_per_token_to_per_million(
                    pricing.completion.as_deref(),
                ) {
                    Some(v) => v,
                    None => continue,
                };

                // Try to update the model in our registry.
                // The key format is "openrouter:{model_id}".
                registry.update_pricing(
                    "openrouter",
                    &model.id,
                    input_per_million,
                    output_per_million,
                );

                updated_count += 1;
            }
        }

        info!(
            models_fetched = body.data.len(),
            models_updated = updated_count,
            "OpenRouter pricing sync complete"
        );
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Parse a per-token price string (e.g. "0.0000005") into a per-million
    /// token value.
    fn parse_per_token_to_per_million(price_str: Option<&str>) -> Option<f64> {
        let s = price_str?;
        let per_token: f64 = s.parse().ok()?;
        Some(per_token * 1_000_000.0)
    }
}

impl Default for PricingSync {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PricingSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PricingSync").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_per_token_to_per_million_valid() {
        // $0.0000005 per token = $0.50 per million tokens.
        let result = PricingSync::parse_per_token_to_per_million(Some("0.0000005"));
        assert!(result.is_some());
        let value = result.unwrap();
        assert!((value - 0.5).abs() < 1e-6, "Expected 0.5, got {value}");
    }

    #[test]
    fn parse_per_token_to_per_million_zero() {
        let result = PricingSync::parse_per_token_to_per_million(Some("0"));
        assert!(result.is_some());
        assert!((result.unwrap()).abs() < 1e-10);
    }

    #[test]
    fn parse_per_token_to_per_million_none() {
        assert!(PricingSync::parse_per_token_to_per_million(None).is_none());
    }

    #[test]
    fn parse_per_token_to_per_million_invalid() {
        assert!(
            PricingSync::parse_per_token_to_per_million(Some("not_a_number")).is_none()
        );
    }
}
