use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::types::{
    ChatCompletionRequest, ChatCompletionResponse, Choice, Message, Usage,
};

use super::traits::LlmProvider;

// ---------------------------------------------------------------------------
// Gemini-specific request/response types (private to this module)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(rename = "maxOutputTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

/// Provider for Google Gemini (Generative Language API).
///
/// Gemini has its own request/response format, so we convert to and from
/// the OpenAI-compatible types used by the gateway.
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    healthy: AtomicBool,
}

impl std::fmt::Debug for GeminiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiProvider")
            .field("base_url", &self.base_url)
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl GeminiProvider {
    /// Create a new Gemini provider.
    ///
    /// * `api_key` -- Google AI API key.
    pub fn new(api_key: String) -> Self {
        let base_url = "https://generativelanguage.googleapis.com".to_string();

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

    /// Build the generate-content URL for a given model.
    fn generate_url(&self, model_id: &str) -> String {
        format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, model_id, self.api_key
        )
    }

    /// Build the models-list URL (used for health checks).
    fn models_url(&self) -> String {
        format!("{}/v1beta/models?key={}", self.base_url, self.api_key)
    }

    /// Convert an OpenAI-compatible `ChatCompletionRequest` into the Gemini
    /// request format.
    fn to_gemini_request(request: &ChatCompletionRequest) -> GeminiRequest {
        let mut contents: Vec<GeminiContent> = Vec::new();

        for msg in &request.messages {
            let text = match &msg.content {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(v) => v.to_string(),
                None => String::new(),
            };

            let role = match msg.role.as_str() {
                "assistant" => "model".to_string(),
                "system" => {
                    // Gemini does not have a native system role; prepend as a
                    // user message with a "System:" prefix.
                    contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart {
                            text: format!("System: {}", text),
                        }],
                    });
                    continue;
                }
                other => other.to_string(), // "user", "tool", etc.
            };

            contents.push(GeminiContent {
                role,
                parts: vec![GeminiPart { text }],
            });
        }

        // Ensure at least one content entry exists (Gemini rejects empty).
        if contents.is_empty() {
            contents.push(GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: String::new(),
                }],
            });
        }

        let generation_config = if request.temperature.is_some()
            || request.max_tokens.is_some()
            || request.top_p.is_some()
        {
            Some(GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                top_p: request.top_p,
            })
        } else {
            None
        };

        GeminiRequest {
            contents,
            generation_config,
        }
    }

    /// Convert a Gemini response into the OpenAI-compatible
    /// `ChatCompletionResponse`.
    fn from_gemini_response(
        gemini_resp: GeminiResponse,
        model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError> {
        let text = gemini_resp
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref())
            .and_then(|content| content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        let finish_reason = gemini_resp
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.finish_reason.clone())
            .map(|r| {
                // Map Gemini finish reasons to OpenAI equivalents.
                match r.as_str() {
                    "STOP" => "stop".to_string(),
                    "MAX_TOKENS" => "length".to_string(),
                    other => other.to_lowercase(),
                }
            });

        let usage = gemini_resp
            .usage_metadata
            .map(|u| Usage {
                prompt_tokens: u.prompt_token_count.unwrap_or(0),
                completion_tokens: u.candidates_token_count.unwrap_or(0),
                total_tokens: u.total_token_count.unwrap_or(0),
            })
            .unwrap_or(Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            });

        Ok(ChatCompletionResponse {
            id: format!("gemini-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            model: model_id.to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(serde_json::Value::String(text)),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason,
            }],
            usage,
            x_router_metadata: None,
        })
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError> {
        let gemini_request = Self::to_gemini_request(request);

        let response = self
            .client
            .post(self.generate_url(model_id))
            .json(&gemini_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            // Only mark unhealthy on server errors (5xx), not client errors (4xx)
            // like rate limits (429) or bad requests (400).
            if status.is_server_error() {
                self.healthy.store(false, Ordering::Relaxed);
            }
            return Err(AppError::ProviderError(format!(
                "Gemini API error ({}): {}",
                status, error_text
            )));
        }

        self.healthy.store(true, Ordering::Relaxed);

        let gemini_resp: GeminiResponse = response.json().await.map_err(|e| {
            AppError::ProviderError(format!("Failed to parse Gemini response: {e}"))
        })?;

        Self::from_gemini_response(gemini_resp, model_id)
    }

    async fn health_check(&self) -> bool {
        let result = self.client.get(self.models_url()).send().await;
        let is_healthy = matches!(result, Ok(ref r) if r.status().is_success());
        self.healthy.store(is_healthy, Ordering::Relaxed);
        is_healthy
    }
}
