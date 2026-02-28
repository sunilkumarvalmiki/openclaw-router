use openclaw_gateway::optimization::compression::{
    estimate_tokens, normalize_whitespace, PromptCompressor,
};
use openclaw_gateway::types::{ChatCompletionRequest, Message};

/// Helper: build a message.
fn make_message(role: &str, content: &str) -> Message {
    Message {
        role: role.to_string(),
        content: Some(serde_json::Value::String(content.to_string())),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

/// Helper: build a request from messages.
fn make_request(messages: Vec<Message>) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: None,
        messages,
        temperature: None,
        max_tokens: None,
        stream: None,
        top_p: None,
        tools: None,
        tool_choice: None,
        response_format: None,
        x_cost_profile: None,
        x_tier_hint: None,
    }
}

// ---------------------------------------------------------------------------
// Whitespace normalisation
// ---------------------------------------------------------------------------

#[test]
fn test_whitespace_normalization_collapses_spaces() {
    assert_eq!(normalize_whitespace("hello    world"), "hello world");
}

#[test]
fn test_whitespace_normalization_trims_leading_trailing() {
    assert_eq!(normalize_whitespace("  hello  "), "hello");
}

#[test]
fn test_whitespace_normalization_handles_tabs_and_newlines() {
    assert_eq!(normalize_whitespace("a\n\n  b\tc"), "a b c");
}

#[test]
fn test_whitespace_normalization_empty_string() {
    assert_eq!(normalize_whitespace("   "), "");
}

#[test]
fn test_whitespace_normalization_already_clean() {
    assert_eq!(normalize_whitespace("no extra spaces"), "no extra spaces");
}

// ---------------------------------------------------------------------------
// Duplicate system message removal
// ---------------------------------------------------------------------------

#[test]
fn test_duplicate_system_message_removal() {
    let compressor = PromptCompressor::new();
    let mut req = make_request(vec![
        make_message("system", "You are a helpful assistant."),
        make_message("user", "Hello"),
        make_message("system", "You are a helpful assistant."),
        make_message("user", "World"),
    ]);

    compressor.compress(&mut req);

    let system_msgs: Vec<_> = req.messages.iter().filter(|m| m.role == "system").collect();
    assert_eq!(system_msgs.len(), 1, "Only one system message should remain");
}

#[test]
fn test_keeps_last_system_message() {
    let compressor = PromptCompressor::new();
    let mut req = make_request(vec![
        make_message("system", "First system prompt."),
        make_message("system", "Second system prompt."),
        make_message("user", "Hi"),
    ]);

    compressor.compress(&mut req);

    let sys = req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .expect("System message should exist");
    let content = sys.content.as_ref().unwrap().as_str().unwrap();
    assert_eq!(content, "Second system prompt.");
}

#[test]
fn test_single_system_message_preserved() {
    let compressor = PromptCompressor::new();
    let mut req = make_request(vec![
        make_message("system", "Be helpful."),
        make_message("user", "Hello"),
    ]);

    compressor.compress(&mut req);

    let system_count = req.messages.iter().filter(|m| m.role == "system").count();
    assert_eq!(system_count, 1);
}

// ---------------------------------------------------------------------------
// Empty message removal
// ---------------------------------------------------------------------------

#[test]
fn test_empty_message_removal() {
    let compressor = PromptCompressor::new();
    let mut req = make_request(vec![
        make_message("user", ""),
        make_message("user", "   "),
        make_message("user", "Real message"),
    ]);

    compressor.compress(&mut req);

    assert_eq!(req.messages.len(), 1);
    let content = req.messages[0].content.as_ref().unwrap().as_str().unwrap();
    assert_eq!(content, "Real message");
}

#[test]
fn test_null_content_removed() {
    let compressor = PromptCompressor::new();
    let mut req = make_request(vec![
        Message {
            role: "user".to_string(),
            content: Some(serde_json::Value::Null),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        make_message("user", "Kept"),
    ]);

    compressor.compress(&mut req);

    assert_eq!(req.messages.len(), 1);
}

// ---------------------------------------------------------------------------
// Compression ratio
// ---------------------------------------------------------------------------

#[test]
fn test_compression_ratio_for_redundant_content() {
    let compressor = PromptCompressor::new();
    let mut req = make_request(vec![
        make_message("system", "You are a helpful assistant."),
        make_message("system", "You are a helpful assistant."),
        make_message("user", "  Hello    world   "),
        make_message("user", "   "),
    ]);

    let result = compressor.compress(&mut req);

    assert!(
        result.ratio < 1.0,
        "Compression ratio should be < 1.0, got {}",
        result.ratio,
    );
    assert!(
        result.compressed_tokens < result.original_tokens,
        "Compressed tokens ({}) should be less than original ({})",
        result.compressed_tokens,
        result.original_tokens,
    );
}

#[test]
fn test_compression_no_change_for_clean_input() {
    let compressor = PromptCompressor::new();
    let mut req = make_request(vec![
        make_message("system", "Be concise."),
        make_message("user", "Hello"),
    ]);

    let result = compressor.compress(&mut req);

    assert!(
        result.ratio <= 1.0,
        "Clean input should have ratio <= 1.0, got {}",
        result.ratio,
    );
    assert_eq!(req.messages.len(), 2, "No messages should be removed");
}

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

#[test]
fn test_estimate_tokens_empty() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn test_estimate_tokens_basic() {
    // "hello world" = 11 chars / 4 = 2.75, ceil = 3
    assert_eq!(estimate_tokens("hello world"), 3);
}
