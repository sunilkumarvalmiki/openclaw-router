use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::traits::LlmProvider;

/// Provider for locally-running Ollama instances.
///
/// Ollama exposes an OpenAI-compatible API at `/v1/chat/completions`,
/// so we reuse the standard request/response types. No API key is required.
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    healthy: AtomicBool,
}

impl std::fmt::Debug for OllamaProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OllamaProvider")
            .field("endpoint", &self.endpoint)
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    ///
    /// * `endpoint` -- Optional base URL; defaults to `http://localhost:11434`.
    pub fn new(endpoint: Option<String>) -> Self {
        let endpoint = endpoint
            .unwrap_or_else(|| "http://localhost:11434".to_string())
            .trim_end_matches('/')
            .to_string();

        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            endpoint,
            healthy: AtomicBool::new(true),
        }
    }

    fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.endpoint)
    }

    fn tags_url(&self) -> String {
        format!("{}/api/tags", self.endpoint)
    }

    /// Strip custom `x_*` routing fields and force non-streaming before forwarding.
    fn sanitize_request(body: &mut serde_json::Value) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("x_cost_profile");
            obj.remove("x_tier_hint");
            // Force non-streaming — the router collects the full response.
            obj.insert("stream".to_string(), serde_json::Value::Bool(false));
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
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

        let response = self
            .client
            .post(self.completions_url())
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
                "Ollama API error ({}): {}",
                status, error_text
            )));
        }

        self.healthy.store(true, Ordering::Relaxed);

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            AppError::ProviderError(format!("Failed to parse Ollama response: {e}"))
        })?;

        Ok(completion)
    }

    async fn health_check(&self) -> bool {
        let result = self.client.get(self.tags_url()).send().await;
        let is_healthy = matches!(result, Ok(ref r) if r.status().is_success());
        self.healthy.store(is_healthy, Ordering::Relaxed);
        is_healthy
    }
}
