use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Tier
// ---------------------------------------------------------------------------

/// Complexity tier determined by the scoring engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Simple,
    Medium,
    Complex,
    Reasoning,
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tier::Simple => write!(f, "simple"),
            Tier::Medium => write!(f, "medium"),
            Tier::Complex => write!(f, "complex"),
            Tier::Reasoning => write!(f, "reasoning"),
        }
    }
}

// ---------------------------------------------------------------------------
// Cost profile
// ---------------------------------------------------------------------------

/// Cost profile controlling how aggressively the router optimises for price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostProfile {
    /// Cheapest model that meets quality thresholds.
    Eco,
    /// Balanced cost / quality (default).
    Auto,
    /// Highest quality regardless of price.
    Premium,
    /// Free-tier models only.
    Free,
}

impl CostProfile {
    /// Parse from a string (case-insensitive), defaulting to `Auto`.
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "eco" => CostProfile::Eco,
            "auto" | "balanced" => CostProfile::Auto,
            "premium" => CostProfile::Premium,
            "free" => CostProfile::Free,
            _ => CostProfile::Auto,
        }
    }
}

impl fmt::Display for CostProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CostProfile::Eco => write!(f, "eco"),
            CostProfile::Auto => write!(f, "auto"),
            CostProfile::Premium => write!(f, "premium"),
            CostProfile::Free => write!(f, "free"),
        }
    }
}

// ---------------------------------------------------------------------------
// Scoring result (15-dimensional)
// ---------------------------------------------------------------------------

/// Result of the 15-dimensional scoring engine analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringResult {
    // --- Dimensions --------------------------------------------------------
    /// Estimated number of input tokens.
    pub input_tokens: u32,
    /// Estimated context length requirement (normalized 0..100).
    pub context_length: f64,
    /// Percentage of input that looks like source code (0..100).
    pub code_percentage: f64,
    /// Number of distinct programming languages detected.
    pub language_count: u32,
    /// Number of discrete steps / instructions detected.
    pub step_count: u32,
    /// Number of tools provided in the request.
    pub tool_call_count: u32,
    /// Output requirement score (0..100, higher = more structured output).
    pub output_requirement: f64,
    /// How much accuracy / precision is demanded (0..100).
    pub accuracy_need: f64,
    /// How much latency matters (0..100, higher = needs low latency).
    pub latency_need: f64,
    /// How much consistency / determinism is needed (0..100).
    pub consistency_need: f64,
    /// Whether the request needs deep reasoning / chain of thought.
    pub needs_reasoning: bool,
    /// Whether the request contains images / vision content.
    pub needs_vision: bool,
    /// Whether the request uses tool calling.
    pub needs_tools: bool,
    /// Tier of the user making the request.
    pub user_tier: String,
    /// Remaining budget for the user (normalized, default 100.0).
    pub budget_remaining: f64,

    // --- Derived -----------------------------------------------------------
    /// Composite complexity score (0..100).
    pub complexity_score: f64,
    /// Classified tier.
    pub tier: Tier,
    /// Confidence in the tier classification (0..1).
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Model selection
// ---------------------------------------------------------------------------

/// A model chosen by the routing engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    /// Internal model identifier (e.g. "gpt-4o-mini").
    pub model_id: String,
    /// Provider name (e.g. "openai").
    pub provider: String,
    /// Complexity tier this model is suited for.
    pub tier: Tier,
    /// Estimated cost per 1K input tokens (USD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_per_1k_input: Option<f64>,
    /// Estimated cost per 1K output tokens (USD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_per_1k_output: Option<f64>,
    /// Whether the model supports vision.
    pub supports_vision: bool,
    /// Whether the model supports tool calling.
    pub supports_tools: bool,
}

// ---------------------------------------------------------------------------
// Routing decision
// ---------------------------------------------------------------------------

/// Full routing decision produced by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Scoring result from the 15-dimensional analysis.
    pub scoring: ScoringResult,
    /// Cost profile used for this decision.
    pub cost_profile: CostProfile,
    /// Primary model selection.
    pub selected_model: ModelSelection,
    /// Fallback models in order of preference.
    pub fallback_models: Vec<ModelSelection>,
    /// Time taken for the routing decision in microseconds.
    pub decision_time_us: u64,
}
