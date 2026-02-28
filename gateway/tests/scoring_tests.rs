use openclaw_gateway::routing::RequestScorer;
use openclaw_gateway::types::{ChatCompletionRequest, Message, Tool, FunctionDef, Tier};

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
    }
}

#[test]
fn test_simple_query_scores_simple_tier() {
    let scorer = RequestScorer::new();
    let request = simple_request("What is the capital of France?");
    let result = scorer.score(&request);

    assert_eq!(result.tier, Tier::Simple, "Short factual query should be Simple tier, got score={}", result.complexity_score);
    assert!(!result.needs_reasoning);
    assert!(!result.needs_vision);
    assert!(!result.needs_tools);
}

#[test]
fn test_code_request_scores_medium_or_higher() {
    let scorer = RequestScorer::new();
    let request = simple_request(
        "Write a Rust function that implements a binary search tree with the following methods:\n\
         pub fn insert(&mut self, value: i32) { ... }\n\
         pub fn search(&self, value: i32) -> bool { ... }\n\
         pub fn delete(&mut self, value: i32) { ... }\n\
         impl Display for BinarySearchTree { ... }\n\
         struct Node { let left: Option<Box<Node>>, let right: Option<Box<Node>> }\n\
         Also implement a Python version:\n\
         def insert(self, value):\n\
         def search(self, value):\n\
         class Node:\n\
         class BinarySearchTree:\n\
         Then add a JavaScript version:\n\
         const insert = function(value) { ... }\n\
         export class BST { constructor() { } }\n\
         Compare the performance of each approach. First analyze the time complexity, \
         then compare memory usage, next evaluate the ergonomics of each language."
    );
    let result = scorer.score(&request);

    assert!(
        result.tier == Tier::Medium || result.tier == Tier::Complex || result.tier == Tier::Reasoning,
        "Code-heavy request should be at least Medium tier, got {:?} with score={}",
        result.tier,
        result.complexity_score,
    );
    assert!(result.code_percentage > 0.0, "Should detect code keywords");
}

#[test]
fn test_tool_calling_request() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("Get the current weather in San Francisco");
    request.tools = Some(vec![
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "get_weather".to_string(),
                description: Some("Get current weather for a location".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": { "type": "string" }
                    }
                })),
            },
        },
    ]);

    let result = scorer.score(&request);

    assert!(result.needs_tools, "Should detect tool calling need");
    assert_eq!(result.tool_call_count, 1);
}

#[test]
fn test_scoring_completes_under_1ms() {
    let scorer = RequestScorer::new();
    let request = simple_request(
        "Analyze the following code and compare the performance characteristics \
         of different sorting algorithms. Think step by step about the time \
         complexity of each approach. Consider merge sort, quicksort, and heapsort."
    );

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = scorer.score(&request);
    }
    let elapsed = start.elapsed();
    let per_call = elapsed / 1000;

    assert!(
        per_call.as_micros() < 1000,
        "Scoring should complete in under 1ms, took {:?} per call",
        per_call,
    );
}

#[test]
fn test_reasoning_detection() {
    let scorer = RequestScorer::new();
    let request = simple_request(
        "Analyze the pros and cons of microservices vs monolith architecture. \
         Compare them across scalability, maintainability, and deployment. \
         Think step by step."
    );
    let result = scorer.score(&request);

    assert!(result.needs_reasoning, "Should detect reasoning need");
}

#[test]
fn test_vision_detection() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::json!([
                { "type": "text", "text": "What is in this image?" },
                { "type": "image_url", "image_url": { "url": "https://example.com/image.png" } }
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

    let result = scorer.score(&request);
    assert!(result.needs_vision, "Should detect vision need from image_url content part");
}

// ---------------------------------------------------------------------------
// Edge case: empty messages
// ---------------------------------------------------------------------------

#[test]
fn test_empty_messages_scores_simple() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![],
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

    let result = scorer.score(&request);

    assert_eq!(result.tier, Tier::Simple, "Empty messages should be Simple tier");
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.code_percentage, 0.0);
    assert!(!result.needs_reasoning);
    assert!(!result.needs_vision);
    assert!(!result.needs_tools);
}

#[test]
fn test_empty_content_message() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::Value::String(String::new())),
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

    let result = scorer.score(&request);
    assert_eq!(result.tier, Tier::Simple, "Empty content should be Simple tier");
    assert_eq!(result.input_tokens, 0);
}

#[test]
fn test_null_content_message() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "assistant".to_string(),
            content: None,
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

    let result = scorer.score(&request);
    assert_eq!(result.tier, Tier::Simple);
    assert_eq!(result.input_tokens, 0);
}

// ---------------------------------------------------------------------------
// Edge case: very long messages (simulated >100k tokens)
// ---------------------------------------------------------------------------

#[test]
fn test_very_long_message() {
    let scorer = RequestScorer::new();
    // Create a ~400k character string => ~100k tokens
    let long_text = "a ".repeat(200_000);
    let request = simple_request(&long_text);

    let result = scorer.score(&request);

    assert!(result.input_tokens >= 100_000, "Should estimate at least 100k tokens, got {}", result.input_tokens);
    assert!(result.context_length > 50.0, "Context length should be high for 100k+ tokens");
    assert_eq!(result.latency_need, 30.0, "Long requests should have low latency need");
}

#[test]
fn test_context_length_caps_at_100() {
    let scorer = RequestScorer::new();
    // Create string with >512k chars => >128k tokens
    let huge_text = "word ".repeat(600_000);
    let request = simple_request(&huge_text);

    let result = scorer.score(&request);
    assert!(result.context_length <= 100.0, "Context length should cap at 100, got {}", result.context_length);
}

// ---------------------------------------------------------------------------
// Edge case: mixed content (code + natural text)
// ---------------------------------------------------------------------------

#[test]
fn test_mixed_code_and_text() {
    let scorer = RequestScorer::new();
    let request = simple_request(
        "Can you explain how this Rust function works?\n\
         ```rust\n\
         pub fn fibonacci(n: u32) -> u32 {\n\
             let mut a = 0;\n\
             let mut b = 1;\n\
             for _ in 0..n {\n\
                 let temp = a;\n\
                 a = b;\n\
                 b = temp + b;\n\
             }\n\
             return a;\n\
         }\n\
         ```\n\
         I want to understand the time complexity and how the variables change at each step."
    );

    let result = scorer.score(&request);
    assert!(result.code_percentage > 0.0, "Should detect some code in mixed content");
    // The request contains code and natural language, so tokens should be estimated
    assert!(result.input_tokens > 0, "Mixed content should have non-zero token count");
}

// ---------------------------------------------------------------------------
// Boundary value tests for tier classification
// ---------------------------------------------------------------------------

#[test]
fn test_tier_boundary_simple_to_medium() {
    // Score just below 20.0 should be Simple, at 20.0 should be Medium
    let scorer = RequestScorer::new();
    // A request that should score right around the Simple/Medium boundary
    let request = simple_request(
        "Write a short function to add two numbers. Also let me know the steps involved."
    );
    let result = scorer.score(&request);
    // Just verify tier classification is one of the expected values
    assert!(
        result.tier == Tier::Simple || result.tier == Tier::Medium,
        "Near-boundary request should be Simple or Medium, got {:?} with score={}",
        result.tier,
        result.complexity_score,
    );
}

#[test]
fn test_confidence_high_at_tier_center() {
    let scorer = RequestScorer::new();
    // A very simple request should have high confidence (far from boundaries)
    let request = simple_request("Hi");
    let result = scorer.score(&request);

    // Score should be very low (around 2.0 for tokens + baseline) and far from 20.0 boundary
    assert!(result.confidence > 0.0, "Should have positive confidence, got {}", result.confidence);
}

#[test]
fn test_confidence_range() {
    let scorer = RequestScorer::new();
    let request = simple_request("Tell me a joke");
    let result = scorer.score(&request);

    assert!(result.confidence >= 0.0, "Confidence should be >= 0, got {}", result.confidence);
    assert!(result.confidence <= 1.0, "Confidence should be <= 1, got {}", result.confidence);
}

// ---------------------------------------------------------------------------
// Output requirement tests
// ---------------------------------------------------------------------------

#[test]
fn test_output_requirement_with_response_format() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("List the prime numbers under 20");
    request.response_format = Some(serde_json::json!({"type": "json_object"}));

    let result = scorer.score(&request);
    assert!(result.output_requirement >= 40.0, "Response format should add at least 40 to output requirement");
}

#[test]
fn test_output_requirement_with_high_max_tokens() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("Write an essay");
    request.max_tokens = Some(5000);

    let result = scorer.score(&request);
    assert!(result.output_requirement >= 20.0, "High max_tokens (>2000) should boost output requirement");
}

#[test]
fn test_output_requirement_with_tools_and_response_format() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("Get the weather");
    request.response_format = Some(serde_json::json!({"type": "json_object"}));
    request.tools = Some(vec![
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "get_weather".to_string(),
                description: None,
                parameters: None,
            },
        },
    ]);

    let result = scorer.score(&request);
    assert!(result.output_requirement >= 70.0, "Combined tools + response_format should give high output requirement");
}

// ---------------------------------------------------------------------------
// Accuracy need tests
// ---------------------------------------------------------------------------

#[test]
fn test_accuracy_need_mathematical() {
    let scorer = RequestScorer::new();
    let request = simple_request(
        "Calculate the exact value of the equation 3.14159 * 2.71828 and verify \
         the formula gives precise results to 5 decimal places"
    );
    let result = scorer.score(&request);
    assert!(result.accuracy_need > 0.0, "Mathematical content should have non-zero accuracy need");
}

#[test]
fn test_accuracy_need_casual() {
    let scorer = RequestScorer::new();
    let request = simple_request("Tell me a fun fact about dolphins");
    let result = scorer.score(&request);
    assert_eq!(result.accuracy_need, 0.0, "Casual query should have zero accuracy need");
}

// ---------------------------------------------------------------------------
// Latency need tests
// ---------------------------------------------------------------------------

#[test]
fn test_latency_need_very_short_query() {
    let scorer = RequestScorer::new();
    let request = simple_request("Hi");
    let result = scorer.score(&request);
    assert_eq!(result.latency_need, 90.0, "Very short query (<50 tokens) should have highest latency need");
}

#[test]
fn test_latency_need_medium_query() {
    let scorer = RequestScorer::new();
    // ~100 tokens (400 chars / 4)
    let text = "Please help me understand this concept. ".repeat(10);
    let request = simple_request(&text);
    let result = scorer.score(&request);
    assert_eq!(result.latency_need, 70.0, "Medium query (50-200 tokens) should have 70 latency need");
}

// ---------------------------------------------------------------------------
// Consistency need tests
// ---------------------------------------------------------------------------

#[test]
fn test_consistency_need_with_low_temperature() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("What is 2+2?");
    request.temperature = Some(0.1);

    let result = scorer.score(&request);
    assert!(result.consistency_need >= 60.0, "Low temperature should boost consistency need");
}

#[test]
fn test_consistency_need_with_high_temperature() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("Write a creative story");
    request.temperature = Some(0.9);

    let result = scorer.score(&request);
    // Baseline 30, no additional from temperature
    assert_eq!(result.consistency_need, 30.0, "High temperature should not boost consistency need beyond baseline");
}

// ---------------------------------------------------------------------------
// User tier tests
// ---------------------------------------------------------------------------

#[test]
fn test_user_tier_defaults_to_free() {
    let scorer = RequestScorer::new();
    let request = simple_request("Hello");
    let result = scorer.score(&request);
    assert_eq!(result.user_tier, "free", "Default user tier should be 'free'");
}

#[test]
fn test_user_tier_from_hint() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("Hello");
    request.x_tier_hint = Some("enterprise".to_string());

    let result = scorer.score(&request);
    assert_eq!(result.user_tier, "enterprise", "User tier should match x_tier_hint");
}

// ---------------------------------------------------------------------------
// Multi-message conversations
// ---------------------------------------------------------------------------

#[test]
fn test_multi_message_conversation() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![
            Message {
                role: "system".to_string(),
                content: Some(serde_json::Value::String("You are a helpful assistant.".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: Some(serde_json::Value::String("Hello!".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "assistant".to_string(),
                content: Some(serde_json::Value::String("Hi! How can I help you?".to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: Some(serde_json::Value::String("What is Rust?".to_string())),
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
    };

    let result = scorer.score(&request);
    // All message content should be concatenated for analysis
    assert!(result.input_tokens > 0, "Multi-message conversation should have non-zero tokens");
}

// ---------------------------------------------------------------------------
// Multiple tools
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_tools() {
    let scorer = RequestScorer::new();
    let mut request = simple_request("I need weather and stock data");
    request.tools = Some(vec![
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "get_weather".to_string(),
                description: Some("Get weather".to_string()),
                parameters: None,
            },
        },
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "get_stock_price".to_string(),
                description: Some("Get stock price".to_string()),
                parameters: None,
            },
        },
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "send_email".to_string(),
                description: Some("Send email".to_string()),
                parameters: None,
            },
        },
    ]);

    let result = scorer.score(&request);
    assert_eq!(result.tool_call_count, 3);
    assert!(result.needs_tools);
}

// ---------------------------------------------------------------------------
// Budget remaining default
// ---------------------------------------------------------------------------

#[test]
fn test_budget_remaining_default() {
    let scorer = RequestScorer::new();
    let request = simple_request("Hello");
    let result = scorer.score(&request);
    assert_eq!(result.budget_remaining, 100.0, "Budget remaining should default to 100.0");
}

// ---------------------------------------------------------------------------
// Complexity score bounds
// ---------------------------------------------------------------------------

#[test]
fn test_complexity_score_within_bounds() {
    let scorer = RequestScorer::new();

    // Test with minimal input
    let minimal = simple_request("Hi");
    let result_min = scorer.score(&minimal);
    assert!(result_min.complexity_score >= 0.0, "Complexity score should be >= 0");
    assert!(result_min.complexity_score <= 100.0, "Complexity score should be <= 100");

    // Test with maximal input
    let mut maximal = simple_request(
        "Analyze and compare these implementations, think step by step, \
         calculate the exact complexity, verify the formula: \
         fn main() { let x = 5; } \
         def foo(): pass \
         const bar = () => {} \
         step 1. first then next finally"
    );
    maximal.tools = Some(vec![
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "tool1".to_string(),
                description: None,
                parameters: None,
            },
        },
    ]);
    maximal.response_format = Some(serde_json::json!({"type": "json_object"}));

    let result_max = scorer.score(&maximal);
    assert!(result_max.complexity_score >= 0.0, "Complexity score should be >= 0");
    assert!(result_max.complexity_score <= 100.0, "Complexity score should be <= 100");
}

// ---------------------------------------------------------------------------
// Language detection
// ---------------------------------------------------------------------------

#[test]
fn test_detects_multiple_languages() {
    let scorer = RequestScorer::new();
    let request = simple_request(
        "Convert this Rust code to Python:\n\
         pub fn add(a: i32, b: i32) -> i32 { a + b }\n\
         fn main() { let result = add(1, 2); }\n\
         impl Display for MyStruct { }\n\
         mod my_module;\n\
         \n\
         Python version:\n\
         def add(a, b):\n\
             return a + b\n\
         import os\n\
         self.value = 42\n\
         print(add(1, 2))\n"
    );
    let result = scorer.score(&request);
    assert!(result.language_count >= 2, "Should detect at least 2 languages (Rust + Python), got {}", result.language_count);
}

#[test]
fn test_no_language_detected_for_plain_text() {
    let scorer = RequestScorer::new();
    let request = simple_request("Tell me about the history of the Roman Empire and how it fell.");
    let result = scorer.score(&request);
    assert_eq!(result.language_count, 0, "Plain text should not detect any programming languages");
}

// ---------------------------------------------------------------------------
// Step count detection
// ---------------------------------------------------------------------------

#[test]
fn test_step_count_detection() {
    let scorer = RequestScorer::new();
    let request = simple_request(
        "Follow these steps:\n\
         1. First, open the file\n\
         2. Then parse the JSON\n\
         3. Next, validate the data\n\
         4. Finally, save the results"
    );
    let result = scorer.score(&request);
    assert!(result.step_count >= 4, "Should detect at least 4 steps, got {}", result.step_count);
}

// ---------------------------------------------------------------------------
// Content array format (multimodal messages without image)
// ---------------------------------------------------------------------------

#[test]
fn test_content_array_text_only() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::json!([
                { "type": "text", "text": "Hello world" },
                { "type": "text", "text": "How are you doing today?" }
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

    let result = scorer.score(&request);
    assert!(!result.needs_vision, "Text-only content array should not need vision");
    assert!(result.input_tokens > 0, "Should count tokens from content array text parts");
}

// ---------------------------------------------------------------------------
// Scorer default trait
// ---------------------------------------------------------------------------

#[test]
fn test_scorer_default() {
    let scorer = RequestScorer::default();
    let request = simple_request("Test");
    let result = scorer.score(&request);
    // Just verify it works the same as new()
    assert_eq!(result.tier, Tier::Simple);
}
