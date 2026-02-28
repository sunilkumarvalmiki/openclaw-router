use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::traits::LlmProvider;

/// Provider stub for AWS Bedrock.
///
/// AWS Bedrock requires SigV4 request signing which adds significant
/// complexity. For production use, Bedrock models are accessible through
/// OpenRouter — configure `OPENROUTER_API_KEY` and route Bedrock model
/// requests through that provider instead.
///
/// This stub exists so that the provider registry can acknowledge Bedrock
/// as a known backend without pulling in the full AWS SDK dependency chain.
pub struct BedrockProvider {
    region: String,
    healthy: AtomicBool,
}

impl std::fmt::Debug for BedrockProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BedrockProvider")
            .field("region", &self.region)
            .field("healthy", &self.healthy.load(Ordering::Relaxed))
            .finish()
    }
}

impl BedrockProvider {
    /// Create a new Bedrock provider stub.
    ///
    /// * `region` -- AWS region (e.g. "us-east-1").
    /// * `_access_key` -- AWS access key ID (currently unused by the stub).
    /// * `_secret_key` -- AWS secret access key (currently unused by the stub).
    #[allow(unused_variables)]
    pub fn new(region: String, access_key: String, secret_key: String) -> Self {
        Self {
            region,
            healthy: AtomicBool::new(false), // starts unhealthy since it's a stub
        }
    }
}

#[async_trait]
impl LlmProvider for BedrockProvider {
    fn name(&self) -> &str {
        "bedrock"
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    async fn chat_completion(
        &self,
        _request: &ChatCompletionRequest,
        _model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError> {
        Err(AppError::ProviderError(format!(
            "AWS Bedrock provider (region: {}) requires the AWS SDK for SigV4 signing, \
             which is not yet implemented. Use OPENROUTER_API_KEY to access Bedrock models \
             (e.g. \"anthropic/claude-3-opus\") through OpenRouter instead.",
            self.region
        )))
    }

    async fn health_check(&self) -> bool {
        // Stub is never healthy — it cannot serve requests.
        self.healthy.store(false, Ordering::Relaxed);
        false
    }
}
