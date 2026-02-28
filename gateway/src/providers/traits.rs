use async_trait::async_trait;

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

/// Trait that every LLM provider backend must implement.
///
/// Providers are expected to be cheaply cloneable (wrapped in `Arc`) and
/// safe to share across Actix-Web worker threads.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Human-readable provider name (e.g. "openai", "anthropic").
    fn name(&self) -> &str;

    /// Returns `true` if the provider is currently considered healthy.
    fn is_healthy(&self) -> bool;

    /// Send a chat completion request to the provider and return the
    /// normalised response.
    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError>;

    /// Perform a lightweight health check against the provider API.
    /// Returns `true` if the provider responded successfully.
    async fn health_check(&self) -> bool;
}
