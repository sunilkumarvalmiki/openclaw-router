use criterion::{black_box, criterion_group, criterion_main, Criterion};

use openclaw_gateway::routing::scorer::RequestScorer;
use openclaw_gateway::routing::registry::ModelRegistry;
use openclaw_gateway::routing::selector::ModelSelector;
use openclaw_gateway::types::request::{ChatCompletionRequest, Message, Tool, FunctionDef};
use openclaw_gateway::types::routing::{CostProfile, ScoringResult, Tier};

use std::sync::Arc;

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

fn complex_request() -> ChatCompletionRequest {
    let request = ChatCompletionRequest {
        model: Some("gpt-4o".to_string()),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: Some(serde_json::Value::String(
                    "You are an expert code reviewer.".to_string(),
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: Some(serde_json::Value::String(
                    "Analyze the following Rust code for performance issues. \
                     Think step by step about the time complexity. \
                     First examine the data structures, then analyze the algorithms, \
                     next consider memory usage. \
                     ```rust\n\
                     pub fn fibonacci(n: u32) -> u32 {\n\
                         if n <= 1 { return n; }\n\
                         fibonacci(n - 1) + fibonacci(n - 2)\n\
                     }\n\
                     ```\n\
                     Compare this with an iterative approach and calculate the exact \
                     improvement factor."
                        .to_string(),
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        temperature: Some(0.3),
        max_tokens: Some(4096),
        stream: None,
        top_p: None,
        tools: Some(vec![Tool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "run_benchmark".to_string(),
                description: Some("Run a performance benchmark".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "code": { "type": "string" },
                        "iterations": { "type": "number" }
                    }
                })),
            },
        }]),
        tool_choice: None,
        response_format: Some(serde_json::json!({"type": "json_object"})),
        x_cost_profile: Some("premium".to_string()),
        x_tier_hint: Some("pro".to_string()),
    };
    request
}

fn seeded_registry() -> Arc<ModelRegistry> {
    let reg = ModelRegistry::new();
    reg.seed_defaults();
    Arc::new(reg)
}

fn simple_scoring() -> ScoringResult {
    ScoringResult {
        input_tokens: 50,
        context_length: 0.5,
        code_percentage: 0.0,
        language_count: 0,
        step_count: 0,
        tool_call_count: 0,
        output_requirement: 0.0,
        accuracy_need: 0.0,
        latency_need: 90.0,
        consistency_need: 30.0,
        needs_reasoning: false,
        needs_vision: false,
        needs_tools: false,
        user_tier: "free".into(),
        budget_remaining: 100.0,
        complexity_score: 5.0,
        tier: Tier::Simple,
        confidence: 0.85,
    }
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_scoring_simple(c: &mut Criterion) {
    let scorer = RequestScorer::new();
    let request = simple_request("What is the capital of France?");

    c.bench_function("score_simple_request", |b| {
        b.iter(|| scorer.score(black_box(&request)))
    });
}

fn bench_scoring_complex(c: &mut Criterion) {
    let scorer = RequestScorer::new();
    let request = complex_request();

    c.bench_function("score_complex_request", |b| {
        b.iter(|| scorer.score(black_box(&request)))
    });
}

fn bench_scoring_long_input(c: &mut Criterion) {
    let scorer = RequestScorer::new();
    let long_text = "This is a test sentence for benchmarking. ".repeat(1000);
    let request = simple_request(&long_text);

    c.bench_function("score_long_input", |b| {
        b.iter(|| scorer.score(black_box(&request)))
    });
}

fn bench_model_selection(c: &mut Criterion) {
    let registry = seeded_registry();
    let selector = ModelSelector::new(registry);
    let scoring = simple_scoring();

    c.bench_function("model_selection_eco", |b| {
        b.iter(|| selector.select(black_box(&scoring), CostProfile::Eco))
    });
}

fn bench_model_selection_premium(c: &mut Criterion) {
    let registry = seeded_registry();
    let selector = ModelSelector::new(registry);
    let scoring = simple_scoring();

    c.bench_function("model_selection_premium", |b| {
        b.iter(|| selector.select(black_box(&scoring), CostProfile::Premium))
    });
}

fn bench_registry_lookup(c: &mut Criterion) {
    let registry = ModelRegistry::new();
    registry.seed_defaults();

    c.bench_function("registry_get_model", |b| {
        b.iter(|| registry.get_model(black_box("openai"), black_box("gpt-4o")))
    });
}

fn bench_registry_get_models_for_tier(c: &mut Criterion) {
    let registry = ModelRegistry::new();
    registry.seed_defaults();

    c.bench_function("registry_get_models_for_tier", |b| {
        b.iter(|| {
            registry.get_models_for_tier(black_box(Tier::Simple), black_box(CostProfile::Eco))
        })
    });
}

criterion_group!(
    benches,
    bench_scoring_simple,
    bench_scoring_complex,
    bench_scoring_long_input,
    bench_model_selection,
    bench_model_selection_premium,
    bench_registry_lookup,
    bench_registry_get_models_for_tier,
);
criterion_main!(benches);
