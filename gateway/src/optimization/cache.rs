use deadpool_redis::Pool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};

// ---------------------------------------------------------------------------
// CacheStats
// ---------------------------------------------------------------------------

/// Statistics tracked by the request cache via Redis counters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: u64,
}

// ---------------------------------------------------------------------------
// RequestCache
// ---------------------------------------------------------------------------

/// Redis-backed semantic cache for LLM responses.
///
/// Identical requests (same messages, parameters) are served from cache to
/// avoid redundant provider calls. Cache keys are SHA-256 hashes of the
/// normalised request content, so minor whitespace differences still produce
/// a cache hit.
pub struct RequestCache {
    redis_pool: Pool,
    ttl_seconds: u64,
    #[allow(dead_code)]
    max_entries: usize,
}

impl RequestCache {
    /// Create a new `RequestCache`.
    ///
    /// * `redis_pool` — deadpool-redis connection pool.
    /// * `ttl_seconds` — how long a cached response lives (seconds).
    /// * `max_entries` — soft limit on the number of entries (advisory).
    pub fn new(redis_pool: Pool, ttl_seconds: u64, max_entries: usize) -> Self {
        Self {
            redis_pool,
            ttl_seconds,
            max_entries,
        }
    }

    /// Look up a cached response for the given request.
    ///
    /// Returns `Some(response)` on a cache hit, `None` on a miss.
    /// Redis failures are logged and treated as misses — the gateway never
    /// crashes because of a Redis outage.
    pub async fn get(
        &self,
        request: &ChatCompletionRequest,
    ) -> Option<ChatCompletionResponse> {
        let key = format!("cache:{}", cache_key(request));

        let mut conn = match self.redis_pool.get().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "Redis pool error on cache GET — treating as miss");
                self.increment_misses().await;
                return None;
            }
        };

        let raw: Option<String> = match conn.get(&key).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "Redis GET failed — treating as miss");
                self.increment_misses().await;
                return None;
            }
        };

        match raw {
            Some(json) => match serde_json::from_str::<ChatCompletionResponse>(&json) {
                Ok(resp) => {
                    tracing::info!(cache_key = %key, "Cache HIT");
                    self.increment_hits().await;
                    Some(resp)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to deserialize cached response");
                    self.increment_misses().await;
                    None
                }
            },
            None => {
                tracing::debug!(cache_key = %key, "Cache MISS");
                self.increment_misses().await;
                None
            }
        }
    }

    /// Store a response in the cache, keyed by the request.
    ///
    /// Errors are logged but never propagated as panics — callers can still
    /// return the response to the client even if caching failed.
    pub async fn set(
        &self,
        request: &ChatCompletionRequest,
        response: &ChatCompletionResponse,
    ) -> Result<(), AppError> {
        let key = format!("cache:{}", cache_key(request));

        let json = serde_json::to_string(response)
            .map_err(|e| AppError::CacheError(format!("Serialization error: {e}")))?;

        let mut conn = match self.redis_pool.get().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "Redis pool error on cache SET — skipping");
                return Err(AppError::CacheError(format!("Redis pool error: {e}")));
            }
        };

        let result: Result<(), redis::RedisError> = redis::cmd("SET")
            .arg(&key)
            .arg(&json)
            .arg("EX")
            .arg(self.ttl_seconds)
            .query_async(&mut *conn)
            .await;

        match result {
            Ok(()) => {
                tracing::debug!(cache_key = %key, ttl = self.ttl_seconds, "Cached response");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(error = %e, "Redis SET failed — skipping cache write");
                Err(AppError::CacheError(format!("Redis SET error: {e}")))
            }
        }
    }

    /// Return cache statistics (hits, misses, current key count).
    ///
    /// Redis failures produce zeroed stats rather than errors.
    pub async fn stats(&self) -> CacheStats {
        let mut conn = match self.redis_pool.get().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "Redis pool error fetching cache stats");
                return CacheStats {
                    hits: 0,
                    misses: 0,
                    size: 0,
                };
            }
        };

        let hits: u64 = conn.get("cache:stats:hits").await.unwrap_or(0);
        let misses: u64 = conn.get("cache:stats:misses").await.unwrap_or(0);
        let size: u64 = conn.get("cache:stats:size").await.unwrap_or(0);

        CacheStats { hits, misses, size }
    }

    // -- internal helpers ---------------------------------------------------

    async fn increment_hits(&self) {
        if let Ok(mut conn) = self.redis_pool.get().await {
            let _: Result<u64, _> = conn.incr("cache:stats:hits", 1u64).await;
        }
    }

    async fn increment_misses(&self) {
        if let Ok(mut conn) = self.redis_pool.get().await {
            let _: Result<u64, _> = conn.incr("cache:stats:misses", 1u64).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Cache key generation (public for tests)
// ---------------------------------------------------------------------------

/// Produce a deterministic hex-encoded SHA-256 hash of the normalised request.
///
/// Normalisation:
/// 1. Messages are sorted by role then by content text.
/// 2. Whitespace in string content is collapsed (multiple spaces/newlines
///    become a single space, leading/trailing whitespace is trimmed).
/// 3. Only the semantic payload matters — routing hints and optional
///    parameters that do not change model output are excluded.
pub fn cache_key(request: &ChatCompletionRequest) -> String {
    let mut hasher = Sha256::new();

    // Include model if set.
    if let Some(ref model) = request.model {
        hasher.update(b"model:");
        hasher.update(model.as_bytes());
        hasher.update(b"|");
    }

    // Sort messages for deterministic ordering.
    let mut sorted_msgs: Vec<_> = request
        .messages
        .iter()
        .map(|m| {
            let content_text = extract_text_content(&m.content);
            let normalised = normalize_whitespace(&content_text);
            (m.role.clone(), normalised)
        })
        .collect();
    sorted_msgs.sort();

    for (role, content) in &sorted_msgs {
        hasher.update(b"msg:");
        hasher.update(role.as_bytes());
        hasher.update(b":");
        hasher.update(content.as_bytes());
        hasher.update(b"|");
    }

    // Include temperature if set (affects output).
    if let Some(temp) = request.temperature {
        hasher.update(format!("temp:{temp}|").as_bytes());
    }

    // Include max_tokens if set.
    if let Some(mt) = request.max_tokens {
        hasher.update(format!("max_tokens:{mt}|").as_bytes());
    }

    let result = hasher.finalize();
    hex::encode(result)
}

/// Extract plain text from the `content` field of a message.
///
/// Content can be a simple string or an array of content parts (multi-modal).
/// For arrays we concatenate all "text" parts.
fn extract_text_content(content: &Option<serde_json::Value>) -> String {
    match content {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(parts)) => {
            let mut texts = Vec::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    texts.push(text.to_string());
                }
            }
            texts.join(" ")
        }
        _ => String::new(),
    }
}

/// Collapse runs of whitespace into single spaces and trim.
fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

    fn simple_request(content: &str) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: None,
            messages: vec![make_message("user", content)],
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
        assert_eq!(normalize_whitespace("a\n\n  b\tc"), "a b c");
        assert_eq!(normalize_whitespace("   "), "");
    }

    #[test]
    fn test_cache_key_consistent() {
        let req = simple_request("Hello, world!");
        let k1 = cache_key(&req);
        let k2 = cache_key(&req);
        assert_eq!(k1, k2, "Same request must produce the same cache key");
    }

    #[test]
    fn test_cache_key_differs_for_different_content() {
        let r1 = simple_request("Hello");
        let r2 = simple_request("Goodbye");
        assert_ne!(
            cache_key(&r1),
            cache_key(&r2),
            "Different content must produce different keys"
        );
    }

    #[test]
    fn test_cache_key_normalises_whitespace() {
        let r1 = simple_request("Hello   world");
        let r2 = simple_request("Hello world");
        assert_eq!(
            cache_key(&r1),
            cache_key(&r2),
            "Whitespace differences should not change the key"
        );
    }

    #[test]
    fn test_cache_key_hex_format() {
        let key = cache_key(&simple_request("test"));
        assert_eq!(key.len(), 64, "SHA-256 hex digest should be 64 chars");
        assert!(
            key.chars().all(|c| c.is_ascii_hexdigit()),
            "Key should be hex-encoded"
        );
    }

    #[test]
    fn test_extract_text_content_string() {
        let content = Some(serde_json::Value::String("hello".into()));
        assert_eq!(extract_text_content(&content), "hello");
    }

    #[test]
    fn test_extract_text_content_array() {
        let content = Some(serde_json::json!([
            { "type": "text", "text": "part one" },
            { "type": "image_url", "image_url": { "url": "https://example.com/img.png" } },
            { "type": "text", "text": "part two" }
        ]));
        assert_eq!(extract_text_content(&content), "part one part two");
    }

    #[test]
    fn test_extract_text_content_none() {
        assert_eq!(extract_text_content(&None), "");
    }
}
