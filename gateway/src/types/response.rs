use serde::{Deserialize, Serialize};

use super::request::Message;

// ---------------------------------------------------------------------------
// Chat completion response (OpenAI-compatible)
// ---------------------------------------------------------------------------

/// OpenAI-compatible chat completion response, extended with router metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,

    pub object: String,

    pub created: u64,

    pub model: String,

    pub choices: Vec<Choice>,

    pub usage: Usage,

    /// Router-specific metadata appended by the gateway.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_router_metadata: Option<RouterMetadata>,
}

// ---------------------------------------------------------------------------
// Choice
// ---------------------------------------------------------------------------

/// A single completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,

    pub message: Message,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ---------------------------------------------------------------------------
// Router metadata
// ---------------------------------------------------------------------------

/// Metadata added by the router to every proxied response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterMetadata {
    /// Provider that served the request (e.g. "openai", "anthropic").
    pub provider: String,

    /// Complexity tier assigned by the scoring engine.
    pub tier: String,

    /// Cost profile used for routing.
    pub cost_profile: String,

    /// Estimated cost of this request in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,

    /// What the request would have cost without routing optimisation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_without_router_usd: Option<f64>,

    /// Percentage savings from routing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub savings_percent: Option<f64>,

    /// Whether the response was served from cache.
    pub cache_hit: bool,

    /// Total end-to-end latency in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,

    /// Time spent in the scoring engine in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring_ms: Option<f64>,
}
