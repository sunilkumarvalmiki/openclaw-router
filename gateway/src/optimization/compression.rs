use serde::{Deserialize, Serialize};

use crate::types::ChatCompletionRequest;

// ---------------------------------------------------------------------------
// CompressionResult
// ---------------------------------------------------------------------------

/// Summary of what prompt compression achieved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Estimated token count before compression.
    pub original_tokens: u32,
    /// Estimated token count after compression.
    pub compressed_tokens: u32,
    /// Compression ratio (compressed / original). Values < 1.0 mean savings.
    pub ratio: f64,
}

// ---------------------------------------------------------------------------
// PromptCompressor
// ---------------------------------------------------------------------------

/// Stateless prompt compressor that reduces token usage without changing
/// semantics.
///
/// Strategies applied (in order):
/// 1. Remove messages whose content is empty or whitespace-only.
/// 2. Deduplicate repeated system instructions (keep only the last one).
/// 3. Collapse redundant whitespace inside every remaining message.
pub struct PromptCompressor;

impl PromptCompressor {
    pub fn new() -> Self {
        Self
    }

    /// Compress the request **in place** and return a summary of savings.
    pub fn compress(&self, request: &mut ChatCompletionRequest) -> CompressionResult {
        let original_tokens = self.total_tokens(request);

        // 1. Remove empty messages.
        self.trim_empty_messages(request);

        // 2. Deduplicate system messages (keep only the last one).
        self.dedup_system_messages(request);

        // 3. Normalise whitespace in all remaining messages.
        self.normalize_all_messages(request);

        let compressed_tokens = self.total_tokens(request);
        let ratio = if original_tokens == 0 {
            1.0
        } else {
            compressed_tokens as f64 / original_tokens as f64
        };

        CompressionResult {
            original_tokens,
            compressed_tokens,
            ratio,
        }
    }

    // -- private helpers ----------------------------------------------------

    /// Remove messages with empty or whitespace-only content.
    fn trim_empty_messages(&self, request: &mut ChatCompletionRequest) {
        request.messages.retain(|msg| {
            match &msg.content {
                None => {
                    // Messages without content but with tool_calls are valid.
                    msg.tool_calls.is_some()
                }
                Some(serde_json::Value::String(s)) => !s.trim().is_empty(),
                Some(serde_json::Value::Array(parts)) => {
                    // Keep if at least one text part is non-empty.
                    parts.iter().any(|p| {
                        p.get("text")
                            .and_then(|t| t.as_str())
                            .map_or(false, |t| !t.trim().is_empty())
                    })
                }
                Some(serde_json::Value::Null) => false,
                // Other shapes (objects, numbers) — keep to be safe.
                Some(_) => true,
            }
        });
    }

    /// If there are multiple system messages, keep only the last one.
    fn dedup_system_messages(&self, request: &mut ChatCompletionRequest) {
        let system_count = request
            .messages
            .iter()
            .filter(|m| m.role == "system")
            .count();

        if system_count <= 1 {
            return;
        }

        // Find the index of the *last* system message.
        let last_system_idx = request
            .messages
            .iter()
            .rposition(|m| m.role == "system")
            .expect("count > 1 guarantees existence");

        // Remove all system messages except the last one.
        let mut idx = 0;
        request.messages.retain(|m| {
            let keep = m.role != "system" || idx == last_system_idx;
            if m.role == "system" {
                idx += 1;
            } else {
                idx += 1;
            }
            keep
        });
    }

    /// Collapse whitespace in every message's text content.
    fn normalize_all_messages(&self, request: &mut ChatCompletionRequest) {
        for msg in &mut request.messages {
            match &msg.content {
                Some(serde_json::Value::String(s)) => {
                    let normalised = normalize_whitespace(s);
                    msg.content = Some(serde_json::Value::String(normalised));
                }
                Some(serde_json::Value::Array(parts)) => {
                    let mut new_parts = parts.clone();
                    for part in &mut new_parts {
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            let normalised = normalize_whitespace(text);
                            part.as_object_mut()
                                .unwrap()
                                .insert("text".to_string(), serde_json::Value::String(normalised));
                        }
                    }
                    msg.content = Some(serde_json::Value::Array(new_parts));
                }
                _ => {}
            }
        }
    }

    /// Estimate total tokens across all messages.
    fn total_tokens(&self, request: &ChatCompletionRequest) -> u32 {
        request
            .messages
            .iter()
            .map(|m| {
                let text = match &m.content {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Array(parts)) => parts
                        .iter()
                        .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join(" "),
                    _ => String::new(),
                };
                estimate_tokens(&text)
            })
            .sum()
    }
}

impl Default for PromptCompressor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

/// Collapse runs of whitespace (spaces, tabs, newlines) into single spaces
/// and trim leading/trailing whitespace.
pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Rough token estimate: one token per ~4 characters.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32).div_ceil(4)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    fn make_message(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: Some(serde_json::Value::String(content.to_string())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn make_request(messages: Vec<Message>) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: None,
            messages,
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            x_cost_profile: None,
            x_tier_hint: None,
            x_priority: None,
        }
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("  hello   world  "), "hello world");
        assert_eq!(normalize_whitespace("a\n\nb\tc"), "a b c");
        assert_eq!(normalize_whitespace("already clean"), "already clean");
        assert_eq!(normalize_whitespace("   "), "");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("hello world!"), 3); // 12 chars / 4 = 3
    }

    #[test]
    fn test_trim_empty_messages() {
        let compressor = PromptCompressor::new();
        let mut req = make_request(vec![
            make_message("system", "You are helpful."),
            make_message("user", ""),
            make_message("user", "   "),
            make_message("user", "Hello!"),
        ]);

        compressor.trim_empty_messages(&mut req);

        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, "system");
        assert_eq!(req.messages[1].role, "user");
    }

    #[test]
    fn test_dedup_system_messages() {
        let compressor = PromptCompressor::new();
        let mut req = make_request(vec![
            make_message("system", "First system prompt."),
            make_message("user", "Hello"),
            make_message("system", "Second system prompt."),
            make_message("user", "World"),
        ]);

        compressor.dedup_system_messages(&mut req);

        let system_msgs: Vec<_> = req.messages.iter().filter(|m| m.role == "system").collect();
        assert_eq!(system_msgs.len(), 1, "Only one system message should remain");

        // The remaining system message should be the last one.
        let content = system_msgs[0]
            .content
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(content, "Second system prompt.");
    }

    #[test]
    fn test_compress_reduces_redundant_whitespace() {
        let compressor = PromptCompressor::new();
        let mut req = make_request(vec![make_message(
            "user",
            "Hello    world,   how   are   you?",
        )]);

        let result = compressor.compress(&mut req);

        let compressed_text = req.messages[0]
            .content
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(compressed_text, "Hello world, how are you?");
        assert!(
            result.compressed_tokens <= result.original_tokens,
            "Compressed tokens ({}) should be <= original ({})",
            result.compressed_tokens,
            result.original_tokens,
        );
    }

    #[test]
    fn test_compress_ratio_for_redundant_content() {
        let compressor = PromptCompressor::new();
        let mut req = make_request(vec![
            make_message("system", "You are a helpful assistant."),
            make_message("system", "You are a helpful assistant."),
            make_message("user", "  Hello    world   "),
            make_message("user", "   "),
        ]);

        let result = compressor.compress(&mut req);

        assert!(
            result.ratio < 1.0,
            "Compression ratio should be < 1.0 for redundant content, got {}",
            result.ratio,
        );
    }

    #[test]
    fn test_compress_preserves_non_empty_messages() {
        let compressor = PromptCompressor::new();
        let mut req = make_request(vec![
            make_message("system", "Be helpful."),
            make_message("user", "What is 2+2?"),
        ]);

        compressor.compress(&mut req);

        assert_eq!(req.messages.len(), 2);
    }

    #[test]
    fn test_single_system_message_not_removed() {
        let compressor = PromptCompressor::new();
        let mut req = make_request(vec![
            make_message("system", "Only system."),
            make_message("user", "Hello"),
        ]);

        compressor.dedup_system_messages(&mut req);

        let system_count = req.messages.iter().filter(|m| m.role == "system").count();
        assert_eq!(system_count, 1);
    }
}
