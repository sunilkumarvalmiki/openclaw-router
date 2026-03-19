use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use reqwest::Client;

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::traits::LlmProvider;

/// OpenAI-compatible provider that forwards requests to any
/// OpenAI-compatible API (OpenAI, Azure OpenAI, Ollama, etc.).
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    healthy: AtomicBool,
}

// Manual Debug impl because AtomicBool does not derive Debug in a useful way.
impl std::fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("base_url", &self.base_url)
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl OpenAiProvider {
    /// Create a new OpenAI provider.
    ///
    /// * `api_key` -- Bearer token for the OpenAI API.
    /// * `base_url` -- Optional base URL; defaults to `https://api.openai.com`.
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let base_url = base_url.unwrap_or_else(|| "https://api.openai.com".to_string());

        // Strip any trailing slash for consistent URL building.
        let base_url = base_url.trim_end_matches('/').to_string();

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key,
            base_url,
            healthy: AtomicBool::new(true),
        }
    }

    /// Build the chat completions URL.
    fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    /// Build the models URL (used for health checks).
    fn models_url(&self) -> String {
        format!("{}/v1/models", self.base_url)
    }

    /// Strip custom `x_*` routing fields from the request body before
    /// forwarding to the upstream provider.
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
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError> {
        // Serialize the request, override the model, and strip routing hints.
        let mut body = serde_json::to_value(request)
            .map_err(|e| AppError::Internal(format!("Failed to serialize request: {e}")))?;

        body["model"] = serde_json::Value::String(model_id.to_string());
        Self::sanitize_request(&mut body);

        let response = self
            .client
            .post(self.completions_url())
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            if status.is_server_error() {
                self.healthy.store(false, Ordering::Relaxed);
            }
            return Err(AppError::ProviderError(format!(
                "OpenAI API error ({}): {}",
                status, error_text
            )));
        }

        // Mark healthy on success.
        self.healthy.store(true, Ordering::Relaxed);

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            AppError::ProviderError(format!("Failed to parse OpenAI response: {e}"))
        })?;

        Ok(completion)
    }

    async fn health_check(&self) -> bool {
        let result = self
            .client
            .get(self.models_url())
            .bearer_auth(&self.api_key)
            .send()
            .await;

        let is_healthy = matches!(result, Ok(ref r) if r.status().is_success());
        self.healthy.store(is_healthy, Ordering::Relaxed);
        is_healthy
    }
}
