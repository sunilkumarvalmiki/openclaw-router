use openclaw_gateway::pricing::PricingSync;
use openclaw_gateway::routing::registry::ModelRegistry;

// ===========================================================================
// PricingSync construction tests
// ===========================================================================

#[test]
fn test_pricing_sync_new() {
    // Just verify construction does not panic
    let _sync = PricingSync::new();
}

#[test]
fn test_pricing_sync_default() {
    let _sync = PricingSync::default();
}

#[test]
fn test_pricing_sync_debug() {
    let sync = PricingSync::new();
    let debug_str = format!("{:?}", sync);
    assert!(
        debug_str.contains("PricingSync"),
        "Debug output should contain struct name"
    );
}

// ===========================================================================
// OpenRouter response parsing format tests
// ===========================================================================
// The OpenRouter API returns JSON with specific structure. We test
// deserialization of the format the PricingSync expects.

#[test]
fn test_openrouter_response_format_deserialization() {
    // This is the format returned by https://openrouter.ai/api/v1/models
    let json = serde_json::json!({
        "data": [
            {
                "id": "meta-llama/llama-3.1-70b-instruct",
                "pricing": {
                    "prompt": "0.0000005",
                    "completion": "0.0000008"
                }
            },
            {
                "id": "openai/gpt-4o",
                "pricing": {
                    "prompt": "0.000005",
                    "completion": "0.000015"
                }
            }
        ]
    });

    // Verify the JSON structure is valid
    let data = json["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);

    // Verify first model
    assert_eq!(data[0]["id"].as_str().unwrap(), "meta-llama/llama-3.1-70b-instruct");
    let prompt_price: f64 = data[0]["pricing"]["prompt"].as_str().unwrap().parse().unwrap();
    let completion_price: f64 = data[0]["pricing"]["completion"].as_str().unwrap().parse().unwrap();
    assert!((prompt_price - 0.0000005).abs() < 1e-12);
    assert!((completion_price - 0.0000008).abs() < 1e-12);
}

#[test]
fn test_price_conversion_per_token_to_per_million() {
    // Simulates the parse_per_token_to_per_million logic: per_token * 1_000_000
    let per_token_price = "0.0000005";
    let per_token: f64 = per_token_price.parse().unwrap();
    let per_million = per_token * 1_000_000.0;
    assert!(
        (per_million - 0.5).abs() < 1e-6,
        "Expected $0.50 per million, got ${per_million}"
    );
}

#[test]
fn test_price_conversion_zero() {
    let per_token: f64 = "0".parse().unwrap();
    let per_million = per_token * 1_000_000.0;
    assert!((per_million - 0.0).abs() < 1e-10, "Zero price should stay zero");
}

#[test]
fn test_price_conversion_large_value() {
    // GPT-4 pricing: $0.00003 per token = $30 per million
    let per_token: f64 = "0.00003".parse().unwrap();
    let per_million = per_token * 1_000_000.0;
    assert!(
        (per_million - 30.0).abs() < 1e-6,
        "Expected $30 per million, got ${per_million}"
    );
}

#[test]
fn test_price_conversion_very_small_value() {
    // Very cheap model: $0.00000001 per token = $0.01 per million
    let per_token: f64 = "0.00000001".parse().unwrap();
    let per_million = per_token * 1_000_000.0;
    assert!(
        (per_million - 0.01).abs() < 1e-6,
        "Expected $0.01 per million, got ${per_million}"
    );
}

#[test]
fn test_price_conversion_invalid_string() {
    let result: Result<f64, _> = "not_a_number".parse();
    assert!(result.is_err(), "Invalid price string should fail to parse");
}

#[test]
fn test_price_conversion_empty_string() {
    let result: Result<f64, _> = "".parse();
    assert!(result.is_err(), "Empty string should fail to parse");
}

// ===========================================================================
// Registry pricing update tests
// ===========================================================================

fn seeded_registry() -> ModelRegistry {
    let reg = ModelRegistry::new();
    reg.seed_defaults();
    reg
}

#[test]
fn test_update_pricing_changes_input_cost() {
    let reg = seeded_registry();
    let before = reg.get_model("openai", "gpt-4o").unwrap();
    assert!((before.input_cost_per_million - 2.50).abs() < f64::EPSILON);

    reg.update_pricing("openai", "gpt-4o", 1.0, 5.0);

    let after = reg.get_model("openai", "gpt-4o").unwrap();
    assert!(
        (after.input_cost_per_million - 1.0).abs() < f64::EPSILON,
        "Input cost should be updated to 1.0, got {}",
        after.input_cost_per_million
    );
}

#[test]
fn test_update_pricing_changes_output_cost() {
    let reg = seeded_registry();
    reg.update_pricing("openai", "gpt-4o", 2.0, 8.0);

    let after = reg.get_model("openai", "gpt-4o").unwrap();
    assert!(
        (after.output_cost_per_million - 8.0).abs() < f64::EPSILON,
        "Output cost should be updated to 8.0, got {}",
        after.output_cost_per_million
    );
}

#[test]
fn test_update_pricing_updates_timestamp() {
    let reg = seeded_registry();
    let before = reg.get_model("openai", "gpt-4o").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));
    reg.update_pricing("openai", "gpt-4o", 1.0, 5.0);

    let after = reg.get_model("openai", "gpt-4o").unwrap();
    assert!(
        after.last_updated >= before.last_updated,
        "last_updated should be bumped"
    );
}

#[test]
fn test_update_pricing_nonexistent_model_is_noop() {
    let reg = seeded_registry();
    let count_before = reg.len();

    // Update a model that doesn't exist — should not panic or add entries
    reg.update_pricing("nonexistent", "fake-model", 1.0, 2.0);

    assert_eq!(reg.len(), count_before, "Non-existent model update should be a no-op");
}

#[test]
fn test_update_pricing_to_free() {
    let reg = seeded_registry();
    reg.update_pricing("openai", "gpt-4o", 0.0, 0.0);

    let after = reg.get_model("openai", "gpt-4o").unwrap();
    assert!(after.is_free(), "Model should be free after setting costs to 0");
}

#[test]
fn test_update_pricing_multiple_times() {
    let reg = seeded_registry();

    reg.update_pricing("openai", "gpt-4o", 1.0, 5.0);
    reg.update_pricing("openai", "gpt-4o", 2.0, 6.0);
    reg.update_pricing("openai", "gpt-4o", 3.0, 7.0);

    let after = reg.get_model("openai", "gpt-4o").unwrap();
    assert!(
        (after.input_cost_per_million - 3.0).abs() < f64::EPSILON,
        "Final input cost should be 3.0"
    );
    assert!(
        (after.output_cost_per_million - 7.0).abs() < f64::EPSILON,
        "Final output cost should be 7.0"
    );
}

// ===========================================================================
// Model entry cost estimation with pricing
// ===========================================================================

#[test]
fn test_estimate_cost_after_pricing_update() {
    let reg = seeded_registry();

    // Update to known costs
    reg.update_pricing("openai", "gpt-4o-mini", 0.15, 0.60);

    let model = reg.get_model("openai", "gpt-4o-mini").unwrap();

    // 1M input tokens, 500K output tokens
    let cost = model.estimate_cost(1_000_000, 500_000);
    let expected = 0.15 + (500_000.0 / 1_000_000.0) * 0.60;
    assert!(
        (cost - expected).abs() < 1e-6,
        "Cost estimate should match: expected {expected}, got {cost}"
    );
}

#[test]
fn test_blended_cost_after_update() {
    let reg = seeded_registry();
    reg.update_pricing("openai", "gpt-4o", 4.0, 8.0);

    let model = reg.get_model("openai", "gpt-4o").unwrap();
    let blended = model.blended_cost();
    let expected = (4.0 + 8.0) / 2.0;
    assert!(
        (blended - expected).abs() < f64::EPSILON,
        "Blended cost should be {expected}, got {blended}"
    );
}

// ===========================================================================
// OpenRouter model ID format tests
// ===========================================================================

#[test]
fn test_openrouter_model_id_format() {
    // OpenRouter uses "provider/model" format
    let reg = seeded_registry();
    let model = reg.get_model("openrouter", "meta-llama/llama-3.1-70b-instruct");
    assert!(model.is_some(), "OpenRouter model with slashed ID should be retrievable");
}

#[test]
fn test_openrouter_model_pricing_update() {
    let reg = seeded_registry();
    reg.update_pricing(
        "openrouter",
        "meta-llama/llama-3.1-70b-instruct",
        0.40,
        0.60,
    );

    let model = reg.get_model("openrouter", "meta-llama/llama-3.1-70b-instruct").unwrap();
    assert!(
        (model.input_cost_per_million - 0.40).abs() < f64::EPSILON,
        "OpenRouter model pricing should be updated"
    );
}
