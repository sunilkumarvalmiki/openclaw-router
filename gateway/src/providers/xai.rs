use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::traits::LlmProvider;

/// Provider for xAI (Grok) — <https://api.x.ai>.
///
/// xAI exposes an OpenAI-compatible API, so the implementation is
/// straightforward: set the model, forward the request, parse the response.
pub struct XaiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    healthy: AtomicBool,
}

impl std::fmt::Debug for XaiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XaiProvider")
            .field("base_url", &self.base_url)
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl XaiProvider {
    /// Create a new xAI provider.
    ///
    /// * `api_key` -- xAI API key.
    pub fn new(api_key: String) -> Self {
        let base_url = "https://api.x.ai".to_string();

        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key,
            base_url,
            healthy: AtomicBool::new(true),
        }
    }

    fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    fn models_url(&self) -> String {
        format!("{}/v1/models", self.base_url)
    }

    fn strip_custom_fields(body: &mut serde_json::Value) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("x_cost_profile");
            obj.remove("x_tier_hint");
        }
    }
}

#[async_trait]
impl LlmProvider for XaiProvider {
    fn name(&self) -> &str {
        "xai"
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
        Self::strip_custom_fields(&mut body);

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
            self.healthy.store(false, Ordering::Relaxed);
            return Err(AppError::ProviderError(format!(
                "xAI API error ({}): {}",
                status, error_text
            )));
        }

        self.healthy.store(true, Ordering::Relaxed);

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            AppError::ProviderError(format!("Failed to parse xAI response: {e}"))
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
