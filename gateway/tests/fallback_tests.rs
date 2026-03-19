use std::sync::Arc;

use async_trait::async_trait;

use openclaw_gateway::error::AppError;
use openclaw_gateway::providers::LlmProvider;
use openclaw_gateway::resilience::{
    CircuitBreakerState, FallbackChain, ProviderCircuitBreakers, RetryConfig,
};
use openclaw_gateway::types::{
    ChatCompletionRequest, ChatCompletionResponse, Choice, Message, Usage,
};

// ===========================================================================
// Mock provider
// ===========================================================================

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
            id: format!("test-{model}"),
            object: "chat.completion".to_string(),
            created: 0,
            model: model.to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(serde_json::Value::String("response".to_string())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
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
        model: Some("test-model".to_string()),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::Value::String("hello".to_string())),
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

fn no_retry_config() -> RetryConfig {
    RetryConfig {
        max_retries: 0,
        base_delay_ms: 10,
        max_delay_ms: 50,
    }
}

// ===========================================================================
// Fallback chain ordering tests
// ===========================================================================

#[tokio::test]
async fn test_chain_uses_first_provider_when_healthy() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(
        vec![
            ("primary".into(), MockProvider::success("primary")),
            ("secondary".into(), MockProvider::success("secondary")),
            ("tertiary".into(), MockProvider::success("tertiary")),
        ],
        cbs,
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let (resp, provider) = chain.execute(&req, "test-model").await.unwrap();

    assert_eq!(provider, "primary");
    assert_eq!(resp.model, "test-model");
}

#[tokio::test]
async fn test_chain_falls_back_on_failure() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(
        vec![
            ("primary".into(), MockProvider::failure("primary", "down")),
            ("secondary".into(), MockProvider::success("secondary")),
        ],
        cbs,
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let (_, provider) = chain.execute(&req, "test-model").await.unwrap();
    assert_eq!(provider, "secondary");
}

#[tokio::test]
async fn test_chain_skips_to_third_provider() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(
        vec![
            ("first".into(), MockProvider::failure("first", "error 1")),
            ("second".into(), MockProvider::failure("second", "error 2")),
            ("third".into(), MockProvider::success("third")),
        ],
        cbs,
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let (_, provider) = chain.execute(&req, "test-model").await.unwrap();
    assert_eq!(provider, "third");
}

// ===========================================================================
// Empty chain tests
// ===========================================================================

#[tokio::test]
async fn test_empty_chain_returns_error() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(vec![], cbs).with_retry_config(no_retry_config());

    let req = make_request();
    let result = chain.execute(&req, "test-model").await;
    assert!(result.is_err(), "Empty chain should return error");

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No providers available"),
        "Error should mention no providers: {}",
        err
    );
}

// ===========================================================================
// All providers fail tests
// ===========================================================================

#[tokio::test]
async fn test_all_providers_fail_returns_last_error() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(
        vec![
            ("a".into(), MockProvider::failure("a", "provider a failed")),
            ("b".into(), MockProvider::failure("b", "provider b failed")),
            ("c".into(), MockProvider::failure("c", "provider c failed")),
        ],
        cbs,
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let result = chain.execute(&req, "test-model").await;
    assert!(result.is_err(), "All-failing chain should return error");
}

#[tokio::test]
async fn test_single_provider_failure() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(
        vec![("only".into(), MockProvider::failure("only", "the only provider failed"))],
        cbs,
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let result = chain.execute(&req, "test-model").await;
    assert!(result.is_err());
}

// ===========================================================================
// Circuit breaker integration
// ===========================================================================

#[tokio::test]
async fn test_skips_provider_with_open_circuit_breaker() {
    let cbs = Arc::new(ProviderCircuitBreakers::new(1, 9999));

    // Trip the circuit breaker for "primary"
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
    .with_retry_config(no_retry_config());

    let req = make_request();
    let (_, provider) = chain.execute(&req, "test-model").await.unwrap();
    assert_eq!(provider, "secondary", "Should skip primary due to open circuit breaker");
}

#[tokio::test]
async fn test_all_circuit_breakers_open_returns_error() {
    let cbs = Arc::new(ProviderCircuitBreakers::new(1, 9999));

    // Trip all circuit breakers
    {
        let cb = cbs.get_or_create("a");
        cb.record_failure();
    }
    {
        let cb = cbs.get_or_create("b");
        cb.record_failure();
    }

    let chain = FallbackChain::new(
        vec![
            ("a".into(), MockProvider::success("a")),
            ("b".into(), MockProvider::success("b")),
        ],
        cbs,
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let result = chain.execute(&req, "test-model").await;
    assert!(result.is_err(), "All circuit breakers open should return error");
}

#[tokio::test]
async fn test_successful_request_records_circuit_breaker_success() {
    let cbs = Arc::new(ProviderCircuitBreakers::new(5, 30));

    // Pre-record some failures (but not enough to open)
    {
        let cb = cbs.get_or_create("primary");
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    let chain = FallbackChain::new(
        vec![("primary".into(), MockProvider::success("primary"))],
        cbs.clone(),
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let _ = chain.execute(&req, "test-model").await.unwrap();

    // After success, failure count should be reset
    let cb = cbs.get_or_create("primary");
    assert_eq!(cb.failure_count(), 0, "Success should reset failure count");
    assert_eq!(cb.state(), CircuitBreakerState::Closed);
}

#[tokio::test]
async fn test_failed_request_records_circuit_breaker_failure() {
    let cbs = Arc::new(ProviderCircuitBreakers::new(5, 30));

    let chain = FallbackChain::new(
        vec![("only".into(), MockProvider::failure("only", "server error"))],
        cbs.clone(),
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let _ = chain.execute(&req, "test-model").await;

    // After failure, failure count should increase
    let cb = cbs.get_or_create("only");
    assert!(cb.failure_count() >= 1, "Failure should increment failure count");
}

// ===========================================================================
// Chain with custom retry config
// ===========================================================================

#[tokio::test]
async fn test_chain_with_retry_config() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(
        vec![("only".into(), MockProvider::success("only"))],
        cbs,
    )
    .with_retry_config(RetryConfig {
        max_retries: 5,
        base_delay_ms: 50,
        max_delay_ms: 500,
    });

    let req = make_request();
    let (resp, provider) = chain.execute(&req, "test-model").await.unwrap();
    assert_eq!(provider, "only");
    assert_eq!(resp.model, "test-model");
}

// ===========================================================================
// Response correctness
// ===========================================================================

#[tokio::test]
async fn test_response_has_correct_model_id() {
    let cbs = Arc::new(ProviderCircuitBreakers::default());
    let chain = FallbackChain::new(
        vec![("provider".into(), MockProvider::success("provider"))],
        cbs,
    )
    .with_retry_config(no_retry_config());

    let req = make_request();
    let (resp, _) = chain.execute(&req, "custom-model-id").await.unwrap();
    assert_eq!(resp.model, "custom-model-id");
}
