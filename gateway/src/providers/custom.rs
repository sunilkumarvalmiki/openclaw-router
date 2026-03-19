use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::traits::LlmProvider;

/// A generic provider for any OpenAI-compatible API endpoint.
///
/// This provider can be used to connect to self-hosted models (vLLM,
/// text-generation-inference, etc.) or third-party services that expose
/// the standard `/v1/chat/completions` endpoint.
pub struct CustomProvider {
    client: Client,
    name_str: String,
    base_url: String,
    api_key: Option<String>,
    healthy: AtomicBool,
}

impl std::fmt::Debug for CustomProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomProvider")
            .field("name", &self.name_str)
            .field("base_url", &self.base_url)
            .field("has_api_key", &self.api_key.is_some())
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl CustomProvider {
    /// Create a new custom provider.
    ///
    /// * `name` -- Human-readable name for this provider instance.
    /// * `base_url` -- Base URL of the OpenAI-compatible API (e.g. `http://localhost:8080`).
    /// * `api_key` -- Optional Bearer token. Omit if the endpoint has no auth.
    pub fn new(name: String, base_url: String, api_key: Option<String>) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();

        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            name_str: name,
            base_url,
            api_key,
            healthy: AtomicBool::new(true),
        }
    }

    fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    fn models_url(&self) -> String {
        format!("{}/v1/models", self.base_url)
    }

    fn sanitize_request(body: &mut serde_json::Value) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("x_cost_profile");
            obj.remove("x_tier_hint");
            obj.remove("x_priority");
            obj.insert("stream".to_string(), serde_json::Value::Bool(false));
        }
    }
}

#[async_trait]
impl LlmProvider for CustomProvider {
    fn name(&self) -> &str {
        &self.name_str
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError> {
        let mut body = serde_json::to_value(request)
            .map_err(|e| AppError::Internal(format!("Failed to serialize request: {e}")))?;

        body["model"] = serde_json::Value::String(model_id.to_string());
        Self::sanitize_request(&mut body);

        let mut req = self.client.post(self.completions_url());

        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let response = req.json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            if status.is_server_error() {
                self.healthy.store(false, Ordering::Relaxed);
            }
            return Err(AppError::ProviderError(format!(
                "Custom provider '{}' API error ({}): {}",
                self.name_str, status, error_text
            )));
        }

        self.healthy.store(true, Ordering::Relaxed);

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            AppError::ProviderError(format!(
                "Failed to parse response from custom provider '{}': {e}",
                self.name_str
            ))
        })?;

        Ok(completion)
    }

    async fn health_check(&self) -> bool {
        let mut req = self.client.get(self.models_url());
        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let result = req.send().await;

        // Custom endpoints may not implement /v1/models, so we are lenient:
        // mark healthy on success OR on connection failure (the endpoint may
        // simply not support a models listing).
        let is_healthy = match result {
            Ok(ref r) => r.status().is_success(),
            Err(_) => {
                // Best-effort: consider the provider healthy even if the models
                // endpoint is not available — the actual completion endpoint
                // may still work.
                true
            }
        };

        self.healthy.store(is_healthy, Ordering::Relaxed);
        is_healthy
    }
}
