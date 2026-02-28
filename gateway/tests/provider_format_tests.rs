use openclaw_gateway::types::{
    ChatCompletionRequest, ChatCompletionResponse, Choice, Message, Usage,
    AnthropicRequest, AnthropicMessage, Tool, FunctionDef, ToolCall, FunctionCall,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn simple_request(content: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: Some("gpt-4o".to_string()),
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
    }
}

fn dummy_response() -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: "chatcmpl-test123".to_string(),
        object: "chat.completion".to_string(),
        created: 1700000000,
        model: "gpt-4o".to_string(),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: "assistant".to_string(),
                content: Some(serde_json::Value::String("Hello!".to_string())),
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

// ===========================================================================
// Request serialization tests
// ===========================================================================

#[test]
fn test_request_serialization_basic() {
    let request = simple_request("Hello, world!");
    let json = serde_json::to_value(&request).unwrap();

    assert_eq!(json["model"], "gpt-4o");
    assert_eq!(json["messages"][0]["role"], "user");
    assert_eq!(json["messages"][0]["content"], "Hello, world!");
}

#[test]
fn test_request_serialization_skips_none_fields() {
    let request = simple_request("test");
    let json = serde_json::to_value(&request).unwrap();
    let obj = json.as_object().unwrap();

    // Optional None fields should be skipped
    assert!(!obj.contains_key("temperature"), "None temperature should be skipped");
    assert!(!obj.contains_key("max_tokens"), "None max_tokens should be skipped");
    assert!(!obj.contains_key("stream"), "None stream should be skipped");
    assert!(!obj.contains_key("top_p"), "None top_p should be skipped");
    assert!(!obj.contains_key("tools"), "None tools should be skipped");
    assert!(!obj.contains_key("tool_choice"), "None tool_choice should be skipped");
    assert!(!obj.contains_key("response_format"), "None response_format should be skipped");
    assert!(!obj.contains_key("x_cost_profile"), "None x_cost_profile should be skipped");
    assert!(!obj.contains_key("x_tier_hint"), "None x_tier_hint should be skipped");
}

#[test]
fn test_request_serialization_includes_present_fields() {
    let mut request = simple_request("test");
    request.temperature = Some(0.7);
    request.max_tokens = Some(1024);
    request.stream = Some(true);
    request.top_p = Some(0.9);
    request.x_cost_profile = Some("eco".to_string());
    request.x_tier_hint = Some("pro".to_string());

    let json = serde_json::to_value(&request).unwrap();

    assert_eq!(json["temperature"], 0.7);
    assert_eq!(json["max_tokens"], 1024);
    assert_eq!(json["stream"], true);
    assert_eq!(json["top_p"], 0.9);
    assert_eq!(json["x_cost_profile"], "eco");
    assert_eq!(json["x_tier_hint"], "pro");
}

#[test]
fn test_request_deserialization_from_json() {
    let json = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            { "role": "user", "content": "Hello" }
        ],
        "temperature": 0.5,
        "max_tokens": 100
    });

    let request: ChatCompletionRequest = serde_json::from_value(json).unwrap();
    assert_eq!(request.model, Some("gpt-4o-mini".to_string()));
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.temperature, Some(0.5));
    assert_eq!(request.max_tokens, Some(100));
    assert!(request.stream.is_none());
}

#[test]
fn test_request_deserialization_minimal() {
    // Only required field is messages
    let json = serde_json::json!({
        "messages": [
            { "role": "user", "content": "Hello" }
        ]
    });

    let request: ChatCompletionRequest = serde_json::from_value(json).unwrap();
    assert!(request.model.is_none());
    assert_eq!(request.messages.len(), 1);
}

#[test]
fn test_request_roundtrip_serialization() {
    let original = simple_request("Round trip test");
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ChatCompletionRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.model, original.model);
    assert_eq!(deserialized.messages.len(), original.messages.len());
}

// ===========================================================================
// Response serialization tests
// ===========================================================================

#[test]
fn test_response_serialization() {
    let response = dummy_response();
    let json = serde_json::to_value(&response).unwrap();

    assert_eq!(json["id"], "chatcmpl-test123");
    assert_eq!(json["object"], "chat.completion");
    assert_eq!(json["created"], 1700000000u64);
    assert_eq!(json["model"], "gpt-4o");
    assert_eq!(json["choices"][0]["index"], 0);
    assert_eq!(json["choices"][0]["message"]["role"], "assistant");
    assert_eq!(json["choices"][0]["message"]["content"], "Hello!");
    assert_eq!(json["choices"][0]["finish_reason"], "stop");
    assert_eq!(json["usage"]["prompt_tokens"], 10);
    assert_eq!(json["usage"]["completion_tokens"], 5);
    assert_eq!(json["usage"]["total_tokens"], 15);
}

#[test]
fn test_response_deserialization() {
    let json = serde_json::json!({
        "id": "chatcmpl-abc123",
        "object": "chat.completion",
        "created": 1700000000u64,
        "model": "gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hi there!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 20,
            "completion_tokens": 10,
            "total_tokens": 30
        }
    });

    let response: ChatCompletionResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.id, "chatcmpl-abc123");
    assert_eq!(response.model, "gpt-4o-mini");
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.usage.total_tokens, 30);
}

#[test]
fn test_response_roundtrip() {
    let original = dummy_response();
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ChatCompletionResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.model, original.model);
    assert_eq!(deserialized.choices.len(), original.choices.len());
    assert_eq!(deserialized.usage.total_tokens, original.usage.total_tokens);
}

// ===========================================================================
// Tool calling format tests
// ===========================================================================

#[test]
fn test_request_with_tools_serialization() {
    let mut request = simple_request("Get the weather");
    request.tools = Some(vec![
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "get_weather".to_string(),
                description: Some("Get current weather".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": { "type": "string" }
                    },
                    "required": ["location"]
                })),
            },
        },
    ]);

    let json = serde_json::to_value(&request).unwrap();
    let tools = json["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["type"], "function");
    assert_eq!(tools[0]["function"]["name"], "get_weather");
}

#[test]
fn test_tool_call_serialization() {
    let tool_call = ToolCall {
        id: "call_abc123".to_string(),
        call_type: "function".to_string(),
        function: FunctionCall {
            name: "get_weather".to_string(),
            arguments: r#"{"location":"San Francisco"}"#.to_string(),
        },
    };

    let json = serde_json::to_value(&tool_call).unwrap();
    assert_eq!(json["id"], "call_abc123");
    assert_eq!(json["type"], "function");
    assert_eq!(json["function"]["name"], "get_weather");
}

#[test]
fn test_message_with_tool_calls() {
    let message = Message {
        role: "assistant".to_string(),
        content: None,
        name: None,
        tool_calls: Some(vec![
            ToolCall {
                id: "call_1".to_string(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: "search".to_string(),
                    arguments: r#"{"query":"rust"}"#.to_string(),
                },
            },
        ]),
        tool_call_id: None,
    };

    let json = serde_json::to_value(&message).unwrap();
    assert_eq!(json["role"], "assistant");
    assert!(json.get("content").is_none() || json["content"].is_null());
    assert_eq!(json["tool_calls"][0]["function"]["name"], "search");
}

#[test]
fn test_tool_response_message() {
    let message = Message {
        role: "tool".to_string(),
        content: Some(serde_json::Value::String(r#"{"temp": 72}"#.to_string())),
        name: Some("get_weather".to_string()),
        tool_calls: None,
        tool_call_id: Some("call_abc123".to_string()),
    };

    let json = serde_json::to_value(&message).unwrap();
    assert_eq!(json["role"], "tool");
    assert_eq!(json["tool_call_id"], "call_abc123");
    assert_eq!(json["name"], "get_weather");
}

// ===========================================================================
// Anthropic request format tests
// ===========================================================================

#[test]
fn test_anthropic_request_serialization() {
    let request = AnthropicRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: serde_json::Value::String("Hello, Claude!".to_string()),
        }],
        max_tokens: 1024,
        system: Some("You are a helpful assistant.".to_string()),
        temperature: Some(0.7),
        stream: None,
        tools: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["model"], "claude-3-5-sonnet-20241022");
    assert_eq!(json["messages"][0]["role"], "user");
    assert_eq!(json["max_tokens"], 1024);
    assert_eq!(json["system"], "You are a helpful assistant.");
    assert_eq!(json["temperature"], 0.7);
}

#[test]
fn test_anthropic_request_skips_none_fields() {
    let request = AnthropicRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: serde_json::Value::String("Hello".to_string()),
        }],
        max_tokens: 256,
        system: None,
        temperature: None,
        stream: None,
        tools: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    let obj = json.as_object().unwrap();
    assert!(!obj.contains_key("system"));
    assert!(!obj.contains_key("temperature"));
    assert!(!obj.contains_key("stream"));
    assert!(!obj.contains_key("tools"));
}

// ===========================================================================
// Multimodal content format tests
// ===========================================================================

#[test]
fn test_vision_content_format() {
    let request = ChatCompletionRequest {
        model: Some("gpt-4o".to_string()),
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::json!([
                {
                    "type": "text",
                    "text": "What's in this image?"
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": "https://example.com/image.png",
                        "detail": "high"
                    }
                }
            ])),
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
    };

    let json = serde_json::to_value(&request).unwrap();
    let content = json["messages"][0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "image_url");
}

// ===========================================================================
// Custom routing fields strip test
// ===========================================================================

#[test]
fn test_custom_routing_fields_in_serialized_request() {
    let mut request = simple_request("test");
    request.x_cost_profile = Some("eco".to_string());
    request.x_tier_hint = Some("pro".to_string());

    let mut json = serde_json::to_value(&request).unwrap();

    // Simulate what OpenAiProvider::strip_custom_fields does
    if let Some(obj) = json.as_object_mut() {
        obj.remove("x_cost_profile");
        obj.remove("x_tier_hint");
    }

    let obj = json.as_object().unwrap();
    assert!(!obj.contains_key("x_cost_profile"), "Custom fields should be stripped");
    assert!(!obj.contains_key("x_tier_hint"), "Custom fields should be stripped");
    // Standard fields should remain
    assert!(obj.contains_key("model"));
    assert!(obj.contains_key("messages"));
}

// ===========================================================================
// Multiple choices in response
// ===========================================================================

#[test]
fn test_response_multiple_choices() {
    let response = ChatCompletionResponse {
        id: "chatcmpl-multi".to_string(),
        object: "chat.completion".to_string(),
        created: 1700000000,
        model: "gpt-4o".to_string(),
        choices: vec![
            Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(serde_json::Value::String("Option A".to_string())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            },
            Choice {
                index: 1,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(serde_json::Value::String("Option B".to_string())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            },
        ],
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        },
        x_router_metadata: None,
    };

    let json = serde_json::to_value(&response).unwrap();
    let choices = json["choices"].as_array().unwrap();
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0]["index"], 0);
    assert_eq!(choices[1]["index"], 1);
}
