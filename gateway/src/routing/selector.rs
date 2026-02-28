use std::sync::Arc;
use std::time::Instant;

use crate::error::AppError;
use crate::types::routing::{
    CostProfile, ModelSelection, RoutingDecision, ScoringResult, Tier,
};

use super::registry::{ModelEntry, ModelRegistry};

// ---------------------------------------------------------------------------
// ModelSelector
// ---------------------------------------------------------------------------

/// Selects the best model for a given scoring result and cost profile.
///
/// The selector queries the [`ModelRegistry`] to find models that match the
/// request's complexity tier, required capabilities, and budget constraints.
pub struct ModelSelector {
    registry: Arc<ModelRegistry>,
}

impl ModelSelector {
    /// Create a new selector backed by the given registry.
    pub fn new(registry: Arc<ModelRegistry>) -> Self {
        Self { registry }
    }

    /// Select the best model for a scored request.
    ///
    /// Returns a [`RoutingDecision`] containing the primary model, up to 3
    /// fallback models, and timing information.
    pub fn select(
        &self,
        scoring: &ScoringResult,
        profile: CostProfile,
    ) -> Result<RoutingDecision, AppError> {
        let start = Instant::now();

        // 1. Get candidate models for the scored tier and profile.
        let mut candidates = self.registry.get_models_for_tier(scoring.tier, profile);

        // 2. Filter by required features.
        if scoring.needs_vision {
            candidates.retain(|m| m.supports_vision);
        }
        if scoring.needs_tools {
            candidates.retain(|m| m.supports_tools);
        }

        // 3. If no candidates match, try the next tier up.
        if candidates.is_empty() {
            if let Some(next_tier) = Self::next_tier_up(scoring.tier) {
                candidates = self.registry.get_models_for_tier(next_tier, profile);
                if scoring.needs_vision {
                    candidates.retain(|m| m.supports_vision);
                }
                if scoring.needs_tools {
                    candidates.retain(|m| m.supports_tools);
                }
            }
        }

        // Still nothing? Return an error.
        if candidates.is_empty() {
            return Err(AppError::NotFound(
                "No models available matching the request requirements".into(),
            ));
        }

        // 4. Select the primary model based on the cost profile.
        let primary = match profile {
            CostProfile::Eco | CostProfile::Free => {
                // Already sorted cheapest first by the registry.
                candidates[0].clone()
            }
            CostProfile::Auto => {
                // Registry returns balanced sort; first is the best balanced pick.
                candidates[0].clone()
            }
            CostProfile::Premium => {
                // Registry returns highest quality first.
                candidates[0].clone()
            }
        };

        // 5. Pick up to 3 fallback models (different from primary).
        let fallbacks: Vec<ModelEntry> = candidates
            .iter()
            .filter(|m| m.key() != primary.key())
            .take(3)
            .cloned()
            .collect();

        // If we have fewer than 3 fallbacks, try the next tier.
        let mut fallback_selections: Vec<ModelSelection> =
            fallbacks.iter().map(Self::entry_to_selection).collect();

        if fallback_selections.len() < 3 {
            if let Some(next_tier) = Self::next_tier_up(scoring.tier) {
                let next_tier_models =
                    self.registry.get_models_for_tier(next_tier, profile);
                for m in next_tier_models {
                    if fallback_selections.len() >= 3 {
                        break;
                    }
                    if m.key() != primary.key()
                        && !fallback_selections.iter().any(|f| {
                            f.model_id == m.model_id && f.provider == m.provider
                        })
                    {
                        if scoring.needs_vision && !m.supports_vision {
                            continue;
                        }
                        if scoring.needs_tools && !m.supports_tools {
                            continue;
                        }
                        fallback_selections.push(Self::entry_to_selection(&m));
                    }
                }
            }
        }

        let decision_time_us = start.elapsed().as_micros() as u64;

        Ok(RoutingDecision {
            scoring: scoring.clone(),
            cost_profile: profile,
            selected_model: Self::entry_to_selection(&primary),
            fallback_models: fallback_selections,
            decision_time_us,
        })
    }

    /// Estimate the total cost (in USD) for a request with the given token
    /// counts routed to a specific model.
    pub fn calculate_cost(
        model: &ModelEntry,
        input_tokens: u32,
        estimated_output_tokens: u32,
    ) -> f64 {
        model.estimate_cost(input_tokens, estimated_output_tokens)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Convert a [`ModelEntry`] into a [`ModelSelection`].
    fn entry_to_selection(entry: &ModelEntry) -> ModelSelection {
        ModelSelection {
            model_id: entry.model_id.clone(),
            provider: entry.provider.clone(),
            tier: entry.tier,
            cost_per_1k_input: Some(entry.input_cost_per_million / 1000.0),
            cost_per_1k_output: Some(entry.output_cost_per_million / 1000.0),
            supports_vision: entry.supports_vision,
            supports_tools: entry.supports_tools,
        }
    }

    /// Return the next higher tier, if one exists.
    fn next_tier_up(tier: Tier) -> Option<Tier> {
        match tier {
            Tier::Simple => Some(Tier::Medium),
            Tier::Medium => Some(Tier::Complex),
            Tier::Complex => Some(Tier::Reasoning),
            Tier::Reasoning => None,
        }
    }
}

impl std::fmt::Debug for ModelSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelSelector")
            .field("registry", &self.registry)
            .finish()
    }
}
