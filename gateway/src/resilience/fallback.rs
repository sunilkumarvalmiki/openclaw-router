use std::sync::Arc;

use crate::error::AppError;
use crate::providers::LlmProvider;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

use super::circuit_breaker::ProviderCircuitBreakers;
use super::retry::{with_retry, RetryConfig};

// ---------------------------------------------------------------------------
// FallbackChain
// ---------------------------------------------------------------------------

/// Ordered chain of LLM providers that implements automatic failover.
///
/// On each request the chain iterates through providers in priority order,
/// skipping any whose circuit breaker is open, and retrying transient errors
/// with exponential backoff.
pub struct FallbackChain {
    /// Providers ordered by priority (highest-priority first).
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
    /// Shared circuit breakers — one per provider name.
    circuit_breakers: Arc<ProviderCircuitBreakers>,
    /// Retry configuration applied to each provider attempt.
    retry_config: RetryConfig,
}

impl FallbackChain {
    /// Build a new fallback chain.
    ///
    /// * `providers` — ordered list of `(name, provider)` pairs.
    /// * `circuit_breakers` — shared per-provider circuit breakers.
    pub fn new(
        providers: Vec<(String, Arc<dyn LlmProvider>)>,
        circuit_breakers: Arc<ProviderCircuitBreakers>,
    ) -> Self {
        Self {
            providers,
            circuit_breakers,
            retry_config: RetryConfig::default(),
        }
    }

    /// Build a fallback chain with a custom retry configuration.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Execute a chat completion request against the chain.
    ///
    /// Returns `(response, provider_name)` on success, or the last error if
    /// every provider failed.
    pub async fn execute(
        &self,
        request: &ChatCompletionRequest,
        model_id: &str,
    ) -> Result<(ChatCompletionResponse, String), AppError> {
        let mut last_error: Option<AppError> = None;

        for (name, provider) in &self.providers {
            // 1. Check circuit breaker
            let cb = self.circuit_breakers.get_or_create(name);
            if !cb.can_execute() {
                tracing::info!(
                    provider = %name,
                    "Skipping provider — circuit breaker is open"
                );
                last_error = Some(AppError::ProviderError(format!(
                    "Circuit breaker open for {name}"
                )));
                continue;
            }
            // Drop the DashMap ref before the async call to avoid holding it
            // across an await point.
            drop(cb);

            // 2. Attempt with retry
            let provider = provider.clone();
            let retry_cfg = self.retry_config.clone();
            let name_clone = name.clone();

            let result = with_retry(&retry_cfg, || {
                let provider = provider.clone();
                async move { provider.chat_completion(request, model_id).await }
            })
            .await;

            match result {
                Ok(response) => {
                    // 3. Success — record and return
                    let cb = self.circuit_breakers.get_or_create(&name_clone);
                    cb.record_success();
                    tracing::info!(provider = %name_clone, "Provider succeeded");
                    return Ok((response, name_clone));
                }
                Err(err) => {
                    // 4. Failure — record and try next
                    let cb = self.circuit_breakers.get_or_create(&name_clone);
                    cb.record_failure();
                    tracing::warn!(
                        provider = %name_clone,
                        error = %err,
                        "Provider failed — trying next in chain"
                    );
                    last_error = Some(err);
                }
            }
        }

        // 5. All providers exhausted
        Err(last_error.unwrap_or_else(|| {
            AppError::ProviderError("No providers available in fallback chain".into())
        }))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::types::{ChatCompletionResponse, Choice, Message, Usage};

    /// A test provider that either succeeds or fails on demand.
    struct MockProvider {
        name: String,
        should_fail: bool,
        fail_message: String,
    }

    impl MockProvider {
        fn success(name: &str) -> Arc<dyn LlmProvider> {
            Arc::new(Self {
                name: name.to_string(),
                should_fail: false,
                fail_message: String::new(),
            })
        }

        fn failure(name: &str, msg: &str) -> Arc<dyn LlmProvider> {
            Arc::new(Self {
                name: name.to_string(),
                should_fail: true,
                fail_message: msg.to_string(),
            })
        }

        fn make_response(model: &str) -> ChatCompletionResponse {
            ChatCompletionResponse {
                id: "test-id".into(),
                object: "chat.completion".into(),
                created: 0,
                model: model.to_string(),
                choices: vec![Choice {
                    index: 0,
                    message: Message {
                        role: "assistant".into(),
                        content: Some(serde_json::Value::String("hello".into())),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    finish_reason: Some("stop".into()),
                }],
                usage: Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                x_router_metadata: None,
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        fn is_healthy(&self) -> bool {
            !self.should_fail
        }

        async fn chat_completion(
            &self,
            _request: &ChatCompletionRequest,
            model_id: &str,
        ) -> Result<ChatCompletionResponse, AppError> {
            if self.should_fail {
                Err(AppError::ProviderError(self.fail_message.clone()))
            } else {
                Ok(Self::make_response(model_id))
            }
        }

        async fn health_check(&self) -> bool {
            !self.should_fail
        }
    }

    fn make_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: Some("test-model".into()),
            messages: vec![Message {
                role: "user".into(),
                content: Some(serde_json::Value::String("hi".into())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            x_cost_profile: None,
            x_tier_hint: None,
            x_priority: None,
        }
    }

    #[tokio::test]
    async fn test_first_provider_succeeds() {
        let cbs = Arc::new(ProviderCircuitBreakers::default());
        let chain = FallbackChain::new(
            vec![
                ("primary".into(), MockProvider::success("primary")),
                ("secondary".into(), MockProvider::success("secondary")),
            ],
            cbs,
        )
        .with_retry_config(RetryConfig {
            max_retries: 0,
            base_delay_ms: 10,
            max_delay_ms: 50,
        });

        let req = make_request();
        let (resp, provider) = chain.execute(&req, "test-model").await.unwrap();
        assert_eq!(provider, "primary");
        assert_eq!(resp.model, "test-model");
    }

    #[tokio::test]
    async fn test_falls_back_to_second_provider() {
        let cbs = Arc::new(ProviderCircuitBreakers::default());
        let chain = FallbackChain::new(
            vec![
                (
                    "primary".into(),
                    MockProvider::failure("primary", "connection refused"),
                ),
                ("secondary".into(), MockProvider::success("secondary")),
            ],
            cbs,
        )
        .with_retry_config(RetryConfig {
            max_retries: 0,
            base_delay_ms: 10,
            max_delay_ms: 50,
        });

        let req = make_request();
        let (_, provider) = chain.execute(&req, "test-model").await.unwrap();
        assert_eq!(provider, "secondary");
    }

    #[tokio::test]
    async fn test_all_providers_fail() {
        let cbs = Arc::new(ProviderCircuitBreakers::default());
        let chain = FallbackChain::new(
            vec![
                (
                    "a".into(),
                    MockProvider::failure("a", "provider a failed"),
                ),
                (
                    "b".into(),
                    MockProvider::failure("b", "provider b failed"),
                ),
            ],
            cbs,
        )
        .with_retry_config(RetryConfig {
            max_retries: 0,
            base_delay_ms: 10,
            max_delay_ms: 50,
        });

        let req = make_request();
        let result = chain.execute(&req, "test-model").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_skips_provider_with_open_circuit_breaker() {
        let cbs = Arc::new(ProviderCircuitBreakers::new(1, 9999));

        // Pre-trip the circuit breaker for "primary"
        {
            let cb = cbs.get_or_create("primary");
            cb.record_failure(); // threshold=1 => opens immediately
        }

        let chain = FallbackChain::new(
            vec![
                ("primary".into(), MockProvider::success("primary")),
                ("secondary".into(), MockProvider::success("secondary")),
            ],
            cbs,
        )
        .with_retry_config(RetryConfig {
            max_retries: 0,
            base_delay_ms: 10,
            max_delay_ms: 50,
        });

        let req = make_request();
        let (_, provider) = chain.execute(&req, "test-model").await.unwrap();
        // Primary should be skipped because its breaker is open.
        assert_eq!(provider, "secondary");
    }
}
