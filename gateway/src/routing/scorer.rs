use crate::types::{ChatCompletionRequest, ScoringResult, Tier};

/// Pure-computation scoring engine that analyses a chat completion request
/// across 15 dimensions and classifies it into a complexity tier.
#[derive(Debug, Clone)]
pub struct RequestScorer;

impl RequestScorer {
    pub fn new() -> Self {
        Self
    }

    /// Score a request across all 15 dimensions and return a [`ScoringResult`].
    ///
    /// This function performs **no I/O** and is designed to complete in
    /// sub-millisecond time.
    pub fn score(&self, request: &ChatCompletionRequest) -> ScoringResult {
        let full_text = Self::extract_text(request);

        // -----------------------------------------------------------------
        // 1. Input tokens (estimated: ~4 characters per token)
        // -----------------------------------------------------------------
        let input_tokens = (full_text.len() as u32) / 4;

        // -----------------------------------------------------------------
        // 2. Context length requirement (normalized 0..100)
        // -----------------------------------------------------------------
        let context_length = Self::score_context_length(input_tokens);

        // -----------------------------------------------------------------
        // 3. Code percentage
        // -----------------------------------------------------------------
        let code_percentage = Self::detect_code_percentage(&full_text);

        // -----------------------------------------------------------------
        // 4. Language count
        // -----------------------------------------------------------------
        let language_count = Self::detect_language_count(&full_text);

        // -----------------------------------------------------------------
        // 5. Step count
        // -----------------------------------------------------------------
        let step_count = Self::detect_step_count(&full_text);

        // -----------------------------------------------------------------
        // 6. Tool call count
        // -----------------------------------------------------------------
        let tool_call_count = request
            .tools
            .as_ref()
            .map(|t| t.len() as u32)
            .unwrap_or(0);

        // -----------------------------------------------------------------
        // 7. Output requirement
        // -----------------------------------------------------------------
        let output_requirement = Self::score_output_requirement(request);

        // -----------------------------------------------------------------
        // 8. Accuracy need
        // -----------------------------------------------------------------
        let accuracy_need = Self::detect_accuracy_need(&full_text);

        // -----------------------------------------------------------------
        // 9. Latency need (short queries need low latency)
        // -----------------------------------------------------------------
        let latency_need = Self::score_latency_need(input_tokens);

        // -----------------------------------------------------------------
        // 10. Consistency need
        // -----------------------------------------------------------------
        let consistency_need = Self::score_consistency_need(request);

        // -----------------------------------------------------------------
        // 11. Needs reasoning
        // -----------------------------------------------------------------
        let needs_reasoning = Self::detect_reasoning_need(&full_text);

        // -----------------------------------------------------------------
        // 12. Needs vision
        // -----------------------------------------------------------------
        let needs_vision = Self::detect_vision_need(request);

        // -----------------------------------------------------------------
        // 13. Needs tools
        // -----------------------------------------------------------------
        let needs_tools = tool_call_count > 0;

        // -----------------------------------------------------------------
        // 14. User tier
        // -----------------------------------------------------------------
        let user_tier = request
            .x_tier_hint
            .clone()
            .unwrap_or_else(|| "free".to_string());

        // -----------------------------------------------------------------
        // 15. Budget remaining
        // -----------------------------------------------------------------
        let budget_remaining = 100.0_f64;

        // -----------------------------------------------------------------
        // Composite complexity score
        // -----------------------------------------------------------------
        let complexity_score = Self::compute_complexity(
            input_tokens,
            context_length,
            code_percentage,
            language_count,
            step_count,
            tool_call_count,
            output_requirement,
            accuracy_need,
            needs_reasoning,
            needs_vision,
            needs_tools,
        );

        // -----------------------------------------------------------------
        // Tier classification
        // -----------------------------------------------------------------
        let tier = Self::classify_tier(complexity_score);

        // Confidence: further from boundaries = more confident
        let confidence = Self::compute_confidence(complexity_score);

        ScoringResult {
            input_tokens,
            context_length,
            code_percentage,
            language_count,
            step_count,
            tool_call_count,
            output_requirement,
            accuracy_need,
            latency_need,
            consistency_need,
            needs_reasoning,
            needs_vision,
            needs_tools,
            user_tier,
            budget_remaining,
            complexity_score,
            tier,
            confidence,
        }
    }

    // =====================================================================
    // Private helpers
    // =====================================================================

    /// Concatenate all message content into a single string for analysis.
    fn extract_text(request: &ChatCompletionRequest) -> String {
        let mut parts: Vec<String> = Vec::new();
        for msg in &request.messages {
            if let Some(content) = &msg.content {
                match content {
                    serde_json::Value::String(s) => parts.push(s.clone()),
                    serde_json::Value::Array(arr) => {
                        for part in arr {
                            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                parts.push(text.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        parts.join(" ")
    }

    /// Context length score: 0..100 based on input token count.
    fn score_context_length(input_tokens: u32) -> f64 {
        // 128k context = 100
        let ratio = input_tokens as f64 / 128_000.0;
        (ratio * 100.0).min(100.0)
    }

    /// Detect the percentage of text that looks like source code.
    fn detect_code_percentage(text: &str) -> f64 {
        if text.is_empty() {
            return 0.0;
        }

        let code_keywords = [
            "fn ", "def ", "class ", "impl ", "const ", "let ", "var ",
            "function ", "return ", "import ", "export ", "pub ", "struct ",
            "enum ", "interface ", "async ", "await ", "for ", "while ",
            "if ", "else ", "match ", "switch ", "case ", "try ", "catch ",
            "=>", "->", "::", "&&", "||", "!=", "==", ">=", "<=",
            "```", "{", "}", "()", "[];",
        ];

        let text_lower = text.to_lowercase();
        let total_words = text.split_whitespace().count().max(1) as f64;
        let code_hits: f64 = code_keywords
            .iter()
            .map(|kw| {
                text_lower.matches(&kw.to_lowercase()).count() as f64
            })
            .sum();

        ((code_hits / total_words) * 100.0).min(100.0)
    }

    /// Count distinct programming languages detected in the text.
    fn detect_language_count(text: &str) -> u32 {
        let text_lower = text.to_lowercase();

        let languages: Vec<(&str, &[&str])> = vec![
            ("rust", &["fn ", "impl ", "let mut", "pub fn", "mod ", "crate", "cargo"]),
            ("python", &["def ", "import ", "self.", "elif ", "print(", "__init__"]),
            ("javascript", &["const ", "let ", "var ", "===", "console.log", "require("]),
            ("typescript", &["interface ", ": string", ": number", ": boolean", "type "]),
            ("go", &["func ", "package ", "go ", "goroutine", "chan "]),
            ("java", &["public class", "private ", "protected ", "System.out", "void "]),
        ];

        let mut count = 0u32;
        for (_lang, keywords) in &languages {
            let matches = keywords
                .iter()
                .filter(|kw| text_lower.contains(&kw.to_lowercase()))
                .count();
            if matches >= 2 {
                count += 1;
            }
        }
        count
    }

    /// Detect number of discrete steps / instructions in the text.
    fn detect_step_count(text: &str) -> u32 {
        let text_lower = text.to_lowercase();

        let step_markers = [
            "step ", "first", "then", "next", "finally", "after that",
            "1.", "2.", "3.", "4.", "5.", "6.", "7.", "8.", "9.", "10.",
            "- ", "* ",
        ];

        step_markers
            .iter()
            .map(|marker| text_lower.matches(marker).count() as u32)
            .sum()
    }

    /// Output requirement: higher when structured output or tools are expected.
    fn score_output_requirement(request: &ChatCompletionRequest) -> f64 {
        let mut score: f64 = 0.0;

        if request.response_format.is_some() {
            score += 40.0;
        }
        if request.tools.is_some() {
            score += 30.0;
        }
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens > 2000 {
                score += 20.0;
            } else if max_tokens > 500 {
                score += 10.0;
            }
        }

        score.min(100.0)
    }

    /// Detect accuracy/precision requirements from the text.
    fn detect_accuracy_need(text: &str) -> f64 {
        let text_lower = text.to_lowercase();

        let accuracy_keywords = [
            "exact", "precise", "calculate", "compute", "accurate",
            "correct", "verify", "validate", "proof", "mathematical",
            "formula", "equation", "decimal", "percentage",
        ];

        let hits = accuracy_keywords
            .iter()
            .filter(|kw| text_lower.contains(*kw))
            .count();

        ((hits as f64) * 15.0).min(100.0)
    }

    /// Short queries need low latency; long ones are more tolerant.
    fn score_latency_need(input_tokens: u32) -> f64 {
        if input_tokens < 50 {
            90.0
        } else if input_tokens < 200 {
            70.0
        } else if input_tokens < 1000 {
            50.0
        } else {
            30.0
        }
    }

    /// Consistency need: structured output or low temperature = high consistency.
    fn score_consistency_need(request: &ChatCompletionRequest) -> f64 {
        let mut score: f64 = 30.0; // baseline

        if request.response_format.is_some() {
            score += 40.0;
        }
        if let Some(temp) = request.temperature {
            if temp < 0.3 {
                score += 30.0;
            } else if temp < 0.7 {
                score += 15.0;
            }
        }

        score.min(100.0)
    }

    /// Detect if the request requires reasoning / chain-of-thought.
    fn detect_reasoning_need(text: &str) -> bool {
        let text_lower = text.to_lowercase();

        let reasoning_keywords = [
            "analyze", "analyse", "compare", "evaluate", "think step by step",
            "reason", "explain why", "pros and cons", "trade-off", "tradeoff",
            "consider", "deduce", "infer", "conclude", "critique",
            "chain of thought", "let's think",
        ];

        reasoning_keywords
            .iter()
            .any(|kw| text_lower.contains(kw))
    }

    /// Detect if any message contains image_url content parts.
    fn detect_vision_need(request: &ChatCompletionRequest) -> bool {
        for msg in &request.messages {
            if let Some(serde_json::Value::Array(parts)) = &msg.content {
                for part in parts {
                    if let Some(t) = part.get("type").and_then(|v| v.as_str()) {
                        if t == "image_url" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Compute composite complexity score (0..100).
    #[allow(clippy::too_many_arguments)]
    fn compute_complexity(
        input_tokens: u32,
        context_length: f64,
        code_percentage: f64,
        language_count: u32,
        step_count: u32,
        tool_call_count: u32,
        output_requirement: f64,
        accuracy_need: f64,
        needs_reasoning: bool,
        needs_vision: bool,
        needs_tools: bool,
    ) -> f64 {
        let mut score = 0.0;

        // Token count contribution (0-15)
        score += if input_tokens < 100 {
            2.0
        } else if input_tokens < 500 {
            5.0
        } else if input_tokens < 2000 {
            8.0
        } else if input_tokens < 8000 {
            12.0
        } else {
            15.0
        };

        // Context length (0-10)
        score += (context_length / 100.0) * 10.0;

        // Code percentage (0-15)
        score += (code_percentage / 100.0) * 15.0;

        // Language diversity (0-10)
        score += (language_count as f64).min(3.0) * 3.33;

        // Step count (0-10)
        score += (step_count as f64).min(10.0);

        // Tool calls (0-10)
        score += (tool_call_count as f64).min(5.0) * 2.0;

        // Output requirement (0-10)
        score += (output_requirement / 100.0) * 10.0;

        // Accuracy need (0-10)
        score += (accuracy_need / 100.0) * 10.0;

        // Reasoning (0-10)
        if needs_reasoning {
            score += 10.0;
        }

        // Vision (0-5)
        if needs_vision {
            score += 5.0;
        }

        // Tools (0-5)
        if needs_tools {
            score += 5.0;
        }

        score.min(100.0)
    }

    /// Classify a complexity score into a tier.
    fn classify_tier(score: f64) -> Tier {
        if score < 20.0 {
            Tier::Simple
        } else if score < 40.0 {
            Tier::Medium
        } else if score < 65.0 {
            Tier::Complex
        } else {
            Tier::Reasoning
        }
    }

    /// Confidence: how far the score is from the nearest boundary.
    fn compute_confidence(score: f64) -> f64 {
        let boundaries = [20.0, 40.0, 65.0];
        let min_distance = boundaries
            .iter()
            .map(|b| (score - b).abs())
            .fold(f64::MAX, f64::min);

        // Normalize: max distance within a band is ~17.5
        (min_distance / 17.5).min(1.0)
    }
}

impl Default for RequestScorer {
    fn default() -> Self {
        Self::new()
    }
}
