use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use reqwest::Client;

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::traits::LlmProvider;

/// GitHub Copilot provider that uses the GitHub Models API.
///
/// The API is OpenAI-compatible and hosted at
/// `https://models.inference.ai.azure.com`.
/// Authentication uses a GitHub personal access token (PAT).
pub struct GitHubCopilotProvider {
    client: Client,
    api_key: String,
    base_url: String,
    healthy: AtomicBool,
}

impl std::fmt::Debug for GitHubCopilotProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubCopilotProvider")
            .field("base_url", &self.base_url)
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl GitHubCopilotProvider {
    /// Create a new GitHub Copilot provider.
    ///
    /// * `api_key` -- GitHub personal access token.
    /// * `base_url` -- Optional override; defaults to GitHub Models API.
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let base_url = base_url
            .unwrap_or_else(|| "https://models.inference.ai.azure.com".to_string());
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

    fn completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// Strip custom routing fields before forwarding.
    fn sanitize_request(body: &mut serde_json::Value) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("x_cost_profile");
            obj.remove("x_tier_hint");
            obj.insert("stream".to_string(), serde_json::Value::Bool(false));
        }
    }
}

#[async_trait]
impl LlmProvider for GitHubCopilotProvider {
    fn name(&self) -> &str {
        "github-copilot"
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
                "GitHub Copilot API error ({}): {}",
                status, error_text
            )));
        }

        self.healthy.store(true, Ordering::Relaxed);

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            AppError::ProviderError(format!("Failed to parse GitHub Copilot response: {e}"))
        })?;

        Ok(completion)
    }

    async fn health_check(&self) -> bool {
        // GitHub Models API does not have a dedicated health endpoint,
        // so we rely on the healthy flag set during request handling.
        self.healthy.load(Ordering::Relaxed)
    }
}
