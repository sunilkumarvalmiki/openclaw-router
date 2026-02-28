use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// OpenAI-compatible chat completion request
// ---------------------------------------------------------------------------

/// OpenAI-compatible chat completion request with custom routing hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// Model identifier (e.g. "gpt-4o", "claude-3-5-sonnet").
    /// When `None`, the router picks the best model for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Conversation messages.
    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,

    // -- Custom routing hints (stripped before forwarding) -------------------

    /// Cost profile hint: "eco", "auto", "premium", or "free".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_cost_profile: Option<String>,

    /// User tier hint: "free", "pro", "enterprise".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_tier_hint: Option<String>,
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

/// A single message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,

    /// Content can be a string or an array of content parts (for vision).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Tool / Function definitions
// ---------------------------------------------------------------------------

/// A tool available for the model to call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Always "function" for now.
    #[serde(rename = "type")]
    pub tool_type: String,

    pub function: FunctionDef,
}

/// Function definition within a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// A tool call emitted by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,

    #[serde(rename = "type")]
    pub call_type: String,

    pub function: FunctionCall,
}

/// The function invocation inside a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

// ---------------------------------------------------------------------------
// Anthropic-specific request types
// ---------------------------------------------------------------------------

/// Anthropic Messages API request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicRequest {
    pub model: String,

    pub messages: Vec<AnthropicMessage>,

    pub max_tokens: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

/// A message in the Anthropic Messages API format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: serde_json::Value,
}
