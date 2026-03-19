use openclaw_gateway::optimization::cache::cache_key;
use openclaw_gateway::types::{ChatCompletionRequest, Message};

/// Helper: build a simple single-message request.
fn simple_request(content: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::Value::String(content.to_string())),
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

#[test]
fn test_cache_key_produces_consistent_hashes() {
    let req = simple_request("What is the meaning of life?");
    let key1 = cache_key(&req);
    let key2 = cache_key(&req);
    assert_eq!(key1, key2, "Same request must always produce the same cache key");
}

#[test]
fn test_cache_key_differs_for_different_requests() {
    let r1 = simple_request("Hello, world!");
    let r2 = simple_request("Goodbye, world!");
    assert_ne!(
        cache_key(&r1),
        cache_key(&r2),
        "Different content must produce different cache keys"
    );
}

#[test]
fn test_same_content_with_different_whitespace_produces_same_key() {
    let r1 = simple_request("Hello   world,  how   are   you?");
    let r2 = simple_request("Hello world, how are you?");
    assert_eq!(
        cache_key(&r1),
        cache_key(&r2),
        "Whitespace-normalised content should produce the same key"
    );
}

#[test]
fn test_whitespace_with_newlines_and_tabs() {
    let r1 = simple_request("Hello\n\n  world\t!");
    let r2 = simple_request("Hello world !");
    assert_eq!(
        cache_key(&r1),
        cache_key(&r2),
        "Tabs and newlines should be collapsed just like spaces"
    );
}

#[test]
fn test_cache_key_is_valid_hex_sha256() {
    let key = cache_key(&simple_request("test input"));
    assert_eq!(key.len(), 64, "SHA-256 hex digest is 64 characters");
    assert!(
        key.chars().all(|c| c.is_ascii_hexdigit()),
        "Key must be hex-encoded"
    );
}

#[test]
fn test_cache_key_includes_model_when_set() {
    let mut r1 = simple_request("Hello");
    let r2 = simple_request("Hello");

    r1.model = Some("gpt-4".to_string());

    assert_ne!(
        cache_key(&r1),
        cache_key(&r2),
        "Setting a model should change the cache key"
    );
}

#[test]
fn test_cache_key_includes_temperature() {
    let mut r1 = simple_request("Hello");
    let r2 = simple_request("Hello");

    r1.temperature = Some(0.7);

    assert_ne!(
        cache_key(&r1),
        cache_key(&r2),
        "Setting temperature should change the cache key"
    );
}

#[test]
fn test_cache_key_message_order_independent() {
    let r1 = ChatCompletionRequest {
        model: None,
        messages: vec![
            Message {
                role: "user".to_string(),
                content: Some(serde_json::Value::String("First".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "assistant".to_string(),
                content: Some(serde_json::Value::String("Second".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
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
    };

    let r2 = ChatCompletionRequest {
        model: None,
        messages: vec![
            Message {
                role: "assistant".to_string(),
                content: Some(serde_json::Value::String("Second".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: Some(serde_json::Value::String("First".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
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
    };

    assert_eq!(
        cache_key(&r1),
        cache_key(&r2),
        "Message order should not affect the cache key (messages are sorted)"
    );
}
