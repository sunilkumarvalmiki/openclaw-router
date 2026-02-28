use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::types::routing::{CostProfile, Tier};

// ---------------------------------------------------------------------------
// ModelEntry
// ---------------------------------------------------------------------------

/// A single model known to the routing engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Model identifier at the provider (e.g. "gpt-4o").
    pub model_id: String,
    /// Provider name (e.g. "openai", "gemini", "openrouter").
    pub provider: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Complexity tier this model is best suited for.
    pub tier: Tier,
    /// Cost per 1 million input tokens (USD).
    pub input_cost_per_million: f64,
    /// Cost per 1 million output tokens (USD).
    pub output_cost_per_million: f64,
    /// Maximum context window in tokens.
    pub max_context_length: u32,
    /// Whether the model supports vision / image inputs.
    pub supports_vision: bool,
    /// Whether the model supports tool / function calling.
    pub supports_tools: bool,
    /// Whether the model supports streaming responses.
    pub supports_streaming: bool,
    /// Whether this model is currently enabled for routing.
    pub is_enabled: bool,
    /// Timestamp of the last pricing or capability update.
    pub last_updated: DateTime<Utc>,
}

impl ModelEntry {
    /// Composite key used in the registry: "provider:model_id".
    pub fn key(&self) -> String {
        format!("{}:{}", self.provider, self.model_id)
    }

    /// Total cost for a request with the given token counts (in USD).
    pub fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }

    /// Whether the model is free (zero cost).
    pub fn is_free(&self) -> bool {
        self.input_cost_per_million == 0.0 && self.output_cost_per_million == 0.0
    }

    /// A blended cost metric for sorting: average of input and output cost per
    /// million tokens, giving a single comparable number.
    pub fn blended_cost(&self) -> f64 {
        (self.input_cost_per_million + self.output_cost_per_million) / 2.0
    }
}

// ---------------------------------------------------------------------------
// ModelRegistry
// ---------------------------------------------------------------------------

/// Thread-safe registry of all models known to the routing engine.
///
/// Uses [`DashMap`] for lock-free concurrent reads and writes so that the
/// pricing sync background task can update entries without blocking the
/// hot path.
pub struct ModelRegistry {
    models: DashMap<String, ModelEntry>,
}

impl ModelRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            models: DashMap::new(),
        }
    }

    /// Insert or update a model entry.
    pub fn register(&self, entry: ModelEntry) {
        self.models.insert(entry.key(), entry);
    }

    /// Look up a specific model by provider and model ID.
    pub fn get_model(&self, provider: &str, model_id: &str) -> Option<ModelEntry> {
        let key = format!("{provider}:{model_id}");
        self.models.get(&key).map(|r| r.value().clone())
    }

    /// Update just the pricing fields for an existing model.
    pub fn update_pricing(
        &self,
        provider: &str,
        model_id: &str,
        input_cost: f64,
        output_cost: f64,
    ) {
        let key = format!("{provider}:{model_id}");
        if let Some(mut entry) = self.models.get_mut(&key) {
            entry.input_cost_per_million = input_cost;
            entry.output_cost_per_million = output_cost;
            entry.last_updated = Utc::now();
        }
    }

    /// Return all enabled models suitable for the given tier and cost profile,
    /// sorted according to the profile's strategy.
    pub fn get_models_for_tier(&self, tier: Tier, profile: CostProfile) -> Vec<ModelEntry> {
        let mut models: Vec<ModelEntry> = self
            .models
            .iter()
            .map(|r| r.value().clone())
            .filter(|m| m.is_enabled && Self::model_fits_tier(m, tier))
            .collect();

        match profile {
            CostProfile::Free => {
                // Only free models
                models.retain(|m| m.is_free());
                models.sort_by(|a, b| {
                    a.blended_cost()
                        .partial_cmp(&b.blended_cost())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            CostProfile::Eco => {
                // Sort ascending by cost (cheapest first)
                models.sort_by(|a, b| {
                    a.blended_cost()
                        .partial_cmp(&b.blended_cost())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            CostProfile::Auto => {
                // Balanced: sort by a blend of tier quality and cost.
                // Higher tier models get a boost, but cost still matters.
                models.sort_by(|a, b| {
                    let score_a = Self::balanced_score(a);
                    let score_b = Self::balanced_score(b);
                    score_a
                        .partial_cmp(&score_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            CostProfile::Premium => {
                // Quality first: sort by tier quality descending, then by cost
                // ascending within the same quality level.
                models.sort_by(|a, b| {
                    let quality_a = Self::tier_quality_rank(a.tier);
                    let quality_b = Self::tier_quality_rank(b.tier);
                    // Higher quality first
                    quality_b.cmp(&quality_a).then_with(|| {
                        a.blended_cost()
                            .partial_cmp(&b.blended_cost())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                });
            }
        }

        models
    }

    /// Return the cheapest enabled model capable of handling the given tier.
    pub fn get_cheapest_model(&self, tier: Tier) -> Option<ModelEntry> {
        self.models
            .iter()
            .map(|r| r.value().clone())
            .filter(|m| m.is_enabled && Self::model_fits_tier(m, tier))
            .min_by(|a, b| {
                a.blended_cost()
                    .partial_cmp(&b.blended_cost())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Total number of models in the registry.
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    // -----------------------------------------------------------------------
    // Seed defaults
    // -----------------------------------------------------------------------

    /// Populate the registry with well-known models and their published pricing.
    pub fn seed_defaults(&self) {
        let now = Utc::now();

        let defaults = vec![
            // ----- OpenAI -----
            ModelEntry {
                model_id: "gpt-4o".into(),
                provider: "openai".into(),
                display_name: "GPT-4o".into(),
                tier: Tier::Complex,
                input_cost_per_million: 2.50,
                output_cost_per_million: 10.0,
                max_context_length: 128_000,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "gpt-4o-mini".into(),
                provider: "openai".into(),
                display_name: "GPT-4o Mini".into(),
                tier: Tier::Medium,
                input_cost_per_million: 0.15,
                output_cost_per_million: 0.60,
                max_context_length: 128_000,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "gpt-3.5-turbo".into(),
                provider: "openai".into(),
                display_name: "GPT-3.5 Turbo".into(),
                tier: Tier::Simple,
                input_cost_per_million: 0.50,
                output_cost_per_million: 1.50,
                max_context_length: 16_385,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            // ----- Gemini -----
            ModelEntry {
                model_id: "gemini-2.5-pro".into(),
                provider: "gemini".into(),
                display_name: "Gemini 2.5 Pro".into(),
                tier: Tier::Reasoning,
                input_cost_per_million: 1.25,
                output_cost_per_million: 10.0,
                max_context_length: 1_000_000,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "gemini-2.5-flash".into(),
                provider: "gemini".into(),
                display_name: "Gemini 2.5 Flash".into(),
                tier: Tier::Medium,
                input_cost_per_million: 0.15,
                output_cost_per_million: 0.60,
                max_context_length: 1_000_000,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "gemini-2.5-flash-lite".into(),
                provider: "gemini".into(),
                display_name: "Gemini 2.5 Flash Lite".into(),
                tier: Tier::Simple,
                input_cost_per_million: 0.075,
                output_cost_per_million: 0.30,
                max_context_length: 1_000_000,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            // ----- OpenRouter -----
            ModelEntry {
                model_id: "meta-llama/llama-3.1-70b-instruct".into(),
                provider: "openrouter".into(),
                display_name: "Llama 3.1 70B Instruct".into(),
                tier: Tier::Complex,
                input_cost_per_million: 0.52,
                output_cost_per_million: 0.75,
                max_context_length: 131_072,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "meta-llama/llama-3.1-8b-instruct".into(),
                provider: "openrouter".into(),
                display_name: "Llama 3.1 8B Instruct".into(),
                tier: Tier::Simple,
                input_cost_per_million: 0.055,
                output_cost_per_million: 0.055,
                max_context_length: 131_072,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "mistralai/mistral-large-latest".into(),
                provider: "openrouter".into(),
                display_name: "Mistral Large".into(),
                tier: Tier::Complex,
                input_cost_per_million: 2.0,
                output_cost_per_million: 6.0,
                max_context_length: 128_000,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "qwen/qwen-2.5-72b-instruct".into(),
                provider: "openrouter".into(),
                display_name: "Qwen 2.5 72B Instruct".into(),
                tier: Tier::Complex,
                input_cost_per_million: 0.36,
                output_cost_per_million: 0.40,
                max_context_length: 131_072,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            // ----- Ollama (local, free) -----
            ModelEntry {
                model_id: "llama3.1:8b".into(),
                provider: "ollama".into(),
                display_name: "Llama 3.1 8B (Local)".into(),
                tier: Tier::Simple,
                input_cost_per_million: 0.0,
                output_cost_per_million: 0.0,
                max_context_length: 131_072,
                supports_vision: false,
                supports_tools: false,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "llama3.1:70b".into(),
                provider: "ollama".into(),
                display_name: "Llama 3.1 70B (Local)".into(),
                tier: Tier::Medium,
                input_cost_per_million: 0.0,
                output_cost_per_million: 0.0,
                max_context_length: 131_072,
                supports_vision: false,
                supports_tools: false,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "mistral:7b".into(),
                provider: "ollama".into(),
                display_name: "Mistral 7B (Local)".into(),
                tier: Tier::Simple,
                input_cost_per_million: 0.0,
                output_cost_per_million: 0.0,
                max_context_length: 32_768,
                supports_vision: false,
                supports_tools: false,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            // ----- xAI -----
            ModelEntry {
                model_id: "grok-3".into(),
                provider: "xai".into(),
                display_name: "Grok 3".into(),
                tier: Tier::Reasoning,
                input_cost_per_million: 3.0,
                output_cost_per_million: 15.0,
                max_context_length: 131_072,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "grok-3-mini".into(),
                provider: "xai".into(),
                display_name: "Grok 3 Mini".into(),
                tier: Tier::Medium,
                input_cost_per_million: 0.30,
                output_cost_per_million: 0.50,
                max_context_length: 131_072,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            // ----- DeepSeek -----
            ModelEntry {
                model_id: "deepseek-chat".into(),
                provider: "deepseek".into(),
                display_name: "DeepSeek Chat".into(),
                tier: Tier::Medium,
                input_cost_per_million: 0.27,
                output_cost_per_million: 1.10,
                max_context_length: 64_000,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
            ModelEntry {
                model_id: "deepseek-reasoner".into(),
                provider: "deepseek".into(),
                display_name: "DeepSeek Reasoner".into(),
                tier: Tier::Reasoning,
                input_cost_per_million: 0.55,
                output_cost_per_million: 2.19,
                max_context_length: 64_000,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                is_enabled: true,
                last_updated: now,
            },
        ];

        for entry in defaults {
            self.register(entry);
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Whether a model is capable of handling a given tier.
    /// Models suited for higher tiers can also handle lower tiers.
    fn model_fits_tier(model: &ModelEntry, tier: Tier) -> bool {
        Self::tier_quality_rank(model.tier) >= Self::tier_quality_rank(tier)
    }

    /// Numeric quality ranking for tiers (higher = more capable).
    fn tier_quality_rank(tier: Tier) -> u8 {
        match tier {
            Tier::Simple => 1,
            Tier::Medium => 2,
            Tier::Complex => 3,
            Tier::Reasoning => 4,
        }
    }

    /// Balanced score combining quality and cost (lower is better).
    /// Quality is inverted so that higher quality models score lower.
    fn balanced_score(model: &ModelEntry) -> f64 {
        let quality = Self::tier_quality_rank(model.tier) as f64;
        // Weight: 60% cost, 40% inverse quality.
        // Normalize quality (1-4 range) to cost-comparable range.
        let cost_component = model.blended_cost();
        let quality_component = (5.0 - quality) * 2.0; // 4->2, 3->4, 2->6, 1->8
        cost_component * 0.6 + quality_component * 0.4
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ModelRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelRegistry")
            .field("model_count", &self.models.len())
            .finish()
    }
}
