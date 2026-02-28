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
