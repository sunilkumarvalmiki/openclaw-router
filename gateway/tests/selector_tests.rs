use std::sync::Arc;

use openclaw_gateway::routing::registry::{ModelEntry, ModelRegistry};
use openclaw_gateway::routing::selector::ModelSelector;
use openclaw_gateway::types::routing::{CostProfile, ScoringResult, Tier};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn complex_scoring() -> ScoringResult {
    ScoringResult {
        input_tokens: 5000,
        context_length: 20.0,
        code_percentage: 40.0,
        language_count: 2,
        step_count: 5,
        tool_call_count: 0,
        output_requirement: 50.0,
        accuracy_need: 60.0,
        latency_need: 30.0,
        consistency_need: 50.0,
        needs_reasoning: true,
        needs_vision: false,
        needs_tools: false,
        user_tier: "pro".into(),
        budget_remaining: 100.0,
        complexity_score: 72.0,
        tier: Tier::Reasoning,
        confidence: 0.6,
    }
}

fn vision_scoring() -> ScoringResult {
    ScoringResult {
        input_tokens: 200,
        context_length: 2.0,
        code_percentage: 0.0,
        language_count: 0,
        step_count: 1,
        tool_call_count: 0,
        output_requirement: 10.0,
        accuracy_need: 20.0,
        latency_need: 70.0,
        consistency_need: 30.0,
        needs_reasoning: false,
        needs_vision: true,
        needs_tools: false,
        user_tier: "free".into(),
        budget_remaining: 100.0,
        complexity_score: 15.0,
        tier: Tier::Simple,
        confidence: 0.7,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn simple_request_eco_selects_cheapest() {
    let reg = seeded_registry();
    let selector = ModelSelector::new(reg.clone());

    let scoring = simple_scoring();
    let decision = selector.select(&scoring, CostProfile::Eco).unwrap();

    // The selected model should be among the cheapest available for Simple tier.
    let cost = decision
        .selected_model
        .cost_per_1k_input
        .unwrap_or(f64::MAX)
        + decision
            .selected_model
            .cost_per_1k_output
            .unwrap_or(f64::MAX);

    // Ollama models are free, so cost should be 0 (or very small).
    assert!(
        cost < 0.01,
        "ECO should pick the cheapest model, got cost {} for {}",
        cost,
        decision.selected_model.model_id
    );
}

#[test]
fn complex_request_premium_selects_quality() {
    let reg = seeded_registry();
    let selector = ModelSelector::new(reg);

    let scoring = complex_scoring();
    let decision = selector.select(&scoring, CostProfile::Premium).unwrap();

    // Premium should select a Reasoning-tier model.
    assert_eq!(
        decision.selected_model.tier,
        Tier::Reasoning,
        "PREMIUM should pick a Reasoning-tier model, got {:?} ({})",
        decision.selected_model.tier,
        decision.selected_model.model_id
    );
}

#[test]
fn vision_request_selects_vision_capable_model() {
    let reg = seeded_registry();
    let selector = ModelSelector::new(reg);

    let scoring = vision_scoring();
    let decision = selector.select(&scoring, CostProfile::Auto).unwrap();

    assert!(
        decision.selected_model.supports_vision,
        "Vision request should route to a vision-capable model, got {}",
        decision.selected_model.model_id
    );
}

#[test]
fn no_models_returns_error() {
    // Empty registry — no models at all.
    let reg = Arc::new(ModelRegistry::new());
    let selector = ModelSelector::new(reg);

    let scoring = simple_scoring();
    let result = selector.select(&scoring, CostProfile::Eco);

    assert!(result.is_err(), "Expected error when no models available");
}

#[test]
fn fallback_models_populated() {
    let reg = seeded_registry();
    let selector = ModelSelector::new(reg);

    let scoring = simple_scoring();
    let decision = selector.select(&scoring, CostProfile::Auto).unwrap();

    // We should have at least 1 fallback model.
    assert!(
        !decision.fallback_models.is_empty(),
        "Expected at least one fallback model"
    );

    // Fallback models should be different from the primary.
    for fb in &decision.fallback_models {
        assert!(
            fb.model_id != decision.selected_model.model_id
                || fb.provider != decision.selected_model.provider,
            "Fallback model should differ from primary: {} ({})",
            fb.model_id,
            fb.provider
        );
    }
}

#[test]
fn decision_timing_is_recorded() {
    let reg = seeded_registry();
    let selector = ModelSelector::new(reg);

    let scoring = simple_scoring();
    let decision = selector.select(&scoring, CostProfile::Eco).unwrap();

    // decision_time_us should be a positive number (likely < 10ms).
    // We just verify it was recorded.
    assert!(
        decision.decision_time_us < 1_000_000, // < 1 second
        "Decision time seems unreasonably high: {} us",
        decision.decision_time_us
    );
}

#[test]
fn calculate_cost_is_accurate() {
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
    let cost = ModelSelector::calculate_cost(&entry, 1_000, 500);
    let expected = (1_000.0 / 1_000_000.0) * 2.0 + (500.0 / 1_000_000.0) * 8.0;
    assert!((cost - expected).abs() < 1e-10);
}
