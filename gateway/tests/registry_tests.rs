use openclaw_gateway::routing::registry::{ModelEntry, ModelRegistry};
use openclaw_gateway::types::routing::{CostProfile, Tier};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn seeded_registry() -> ModelRegistry {
    let reg = ModelRegistry::new();
    reg.seed_defaults();
    reg
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn seed_defaults_populates_models() {
    let reg = seeded_registry();
    // We seed at least 17 models across 6 providers.
    assert!(
        reg.len() >= 17,
        "Expected at least 17 models, got {}",
        reg.len()
    );
}

#[test]
fn seed_defaults_contains_known_models() {
    let reg = seeded_registry();

    // OpenAI
    let gpt4o = reg.get_model("openai", "gpt-4o").expect("gpt-4o missing");
    assert_eq!(gpt4o.display_name, "GPT-4o");
    assert!((gpt4o.input_cost_per_million - 2.50).abs() < f64::EPSILON);
    assert!((gpt4o.output_cost_per_million - 10.0).abs() < f64::EPSILON);

    // Gemini
    let flash = reg
        .get_model("gemini", "gemini-2.5-flash")
        .expect("gemini-2.5-flash missing");
    assert!(flash.supports_vision);

    // Ollama (free)
    let local = reg
        .get_model("ollama", "llama3.1:8b")
        .expect("ollama llama3.1:8b missing");
    assert!(local.is_free());

    // xAI
    let grok = reg.get_model("xai", "grok-3").expect("grok-3 missing");
    assert_eq!(grok.tier, Tier::Reasoning);

    // DeepSeek
    let ds = reg
        .get_model("deepseek", "deepseek-chat")
        .expect("deepseek-chat missing");
    assert!(ds.supports_tools);
}

#[test]
fn get_models_for_tier_simple_returns_capable_models() {
    let reg = seeded_registry();
    let models = reg.get_models_for_tier(Tier::Simple, CostProfile::Eco);

    assert!(!models.is_empty(), "Expected some simple-tier models");

    // Every returned model should be capable of Simple tier (rank >= 1)
    for m in &models {
        assert!(
            m.is_enabled,
            "Disabled model returned: {}",
            m.display_name
        );
    }
}

#[test]
fn get_models_for_tier_eco_sorted_cheapest_first() {
    let reg = seeded_registry();
    let models = reg.get_models_for_tier(Tier::Simple, CostProfile::Eco);

    for window in models.windows(2) {
        assert!(
            window[0].blended_cost() <= window[1].blended_cost(),
            "Eco models not sorted by cost: {} ({}) should be <= {} ({})",
            window[0].display_name,
            window[0].blended_cost(),
            window[1].display_name,
            window[1].blended_cost(),
        );
    }
}

#[test]
fn get_models_for_tier_free_returns_only_free_models() {
    let reg = seeded_registry();
    let models = reg.get_models_for_tier(Tier::Simple, CostProfile::Free);

    assert!(!models.is_empty(), "Expected some free models");
    for m in &models {
        assert!(
            m.is_free(),
            "Non-free model returned in Free profile: {} (${}/M input)",
            m.display_name,
            m.input_cost_per_million
        );
    }
}

#[test]
fn get_cheapest_model_returns_cheapest() {
    let reg = seeded_registry();
    let cheapest = reg
        .get_cheapest_model(Tier::Simple)
        .expect("Expected at least one simple-tier model");

    // The cheapest model for Simple should be one of the free Ollama models.
    assert!(
        cheapest.blended_cost() <= 0.06,
        "Cheapest model for Simple has unexpectedly high cost: {} at ${}",
        cheapest.display_name,
        cheapest.blended_cost()
    );
}

#[test]
fn get_cheapest_model_for_reasoning() {
    let reg = seeded_registry();
    let cheapest = reg
        .get_cheapest_model(Tier::Reasoning)
        .expect("Expected at least one reasoning-tier model");

    assert_eq!(cheapest.tier, Tier::Reasoning);
}

#[test]
fn update_pricing_changes_costs() {
    let reg = seeded_registry();

    // Before update
    let before = reg
        .get_model("openai", "gpt-4o")
        .expect("gpt-4o missing");
    assert!((before.input_cost_per_million - 2.50).abs() < f64::EPSILON);

    // Update pricing
    reg.update_pricing("openai", "gpt-4o", 1.00, 5.00);

    // After update
    let after = reg
        .get_model("openai", "gpt-4o")
        .expect("gpt-4o missing after update");
    assert!(
        (after.input_cost_per_million - 1.00).abs() < f64::EPSILON,
        "Input cost not updated: {}",
        after.input_cost_per_million
    );
    assert!(
        (after.output_cost_per_million - 5.00).abs() < f64::EPSILON,
        "Output cost not updated: {}",
        after.output_cost_per_million
    );
    assert!(
        after.last_updated >= before.last_updated,
        "last_updated not bumped"
    );
}

#[test]
fn register_overwrites_existing() {
    let reg = ModelRegistry::new();

    let entry = ModelEntry {
        model_id: "test-model".into(),
        provider: "test".into(),
        display_name: "Test v1".into(),
        tier: Tier::Simple,
        input_cost_per_million: 1.0,
        output_cost_per_million: 2.0,
        max_context_length: 4096,
        supports_vision: false,
        supports_tools: false,
        supports_streaming: true,
        is_enabled: true,
        last_updated: chrono::Utc::now(),
    };

    reg.register(entry);
    assert_eq!(reg.len(), 1);

    let updated = ModelEntry {
        model_id: "test-model".into(),
        provider: "test".into(),
        display_name: "Test v2".into(),
        tier: Tier::Medium,
        input_cost_per_million: 0.5,
        output_cost_per_million: 1.0,
        max_context_length: 8192,
        supports_vision: true,
        supports_tools: true,
        supports_streaming: true,
        is_enabled: true,
        last_updated: chrono::Utc::now(),
    };

    reg.register(updated);
    assert_eq!(reg.len(), 1);

    let fetched = reg.get_model("test", "test-model").unwrap();
    assert_eq!(fetched.display_name, "Test v2");
    assert_eq!(fetched.tier, Tier::Medium);
}

#[test]
fn get_model_returns_none_for_unknown() {
    let reg = seeded_registry();
    assert!(reg.get_model("nonexistent", "fake-model").is_none());
}

#[test]
fn model_entry_estimate_cost() {
    let entry = ModelEntry {
        model_id: "test".into(),
        provider: "test".into(),
        display_name: "Test".into(),
        tier: Tier::Simple,
        input_cost_per_million: 2.0,
        output_cost_per_million: 8.0,
        max_context_length: 4096,
        supports_vision: false,
        supports_tools: false,
        supports_streaming: true,
        is_enabled: true,
        last_updated: chrono::Utc::now(),
    };

    // 1000 input tokens, 500 output tokens
    let cost = entry.estimate_cost(1_000, 500);
    let expected = (1_000.0 / 1_000_000.0) * 2.0 + (500.0 / 1_000_000.0) * 8.0;
    assert!((cost - expected).abs() < 1e-10);
}
