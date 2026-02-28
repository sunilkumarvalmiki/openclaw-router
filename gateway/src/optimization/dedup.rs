use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::watch;

use crate::types::ChatCompletionResponse;

// ---------------------------------------------------------------------------
// DedupResult
// ---------------------------------------------------------------------------

/// Outcome of checking whether this request is the first of its kind or a
/// duplicate that should wait for an in-flight result.
pub enum DedupResult {
    /// This is the first request with this key. The caller should execute the
    /// request and then call [`DedupGuard::complete`] to broadcast the
    /// response to any waiters.
    FirstRequest(DedupGuard),

    /// An identical request is already in flight. Wait on the receiver to
    /// get the response when it is ready.
    Duplicate(DedupReceiver),
}

// ---------------------------------------------------------------------------
// DedupGuard / DedupReceiver
// ---------------------------------------------------------------------------

/// Handle returned for the first request. Dropping it without calling
/// `complete` will unblock waiters with `None`.
pub struct DedupGuard {
    key: String,
    map: Arc<DashMap<String, InFlightEntry>>,
    tx: watch::Sender<Option<ChatCompletionResponse>>,
    completed: bool,
}

impl DedupGuard {
    /// Signal that the request completed and broadcast the response.
    pub fn complete(mut self, response: ChatCompletionResponse) {
        let _ = self.tx.send(Some(response));
        self.completed = true;
        // Entry will be cleaned up on drop.
    }
}

impl Drop for DedupGuard {
    fn drop(&mut self) {
        if !self.completed {
            // The request was abandoned — unblock waiters with None.
            let _ = self.tx.send(None);
        }
        // Remove the entry so future identical requests start fresh.
        self.map.remove(&self.key);
    }
}

/// Handle returned for duplicate requests. Call [`wait`] to block until the
/// first request finishes.
pub struct DedupReceiver {
    rx: watch::Receiver<Option<ChatCompletionResponse>>,
}

impl DedupReceiver {
    /// Wait until the in-flight request completes and return the shared
    /// response. Returns `None` if the first request was abandoned.
    pub async fn wait(mut self) -> Option<ChatCompletionResponse> {
        // `watch::Receiver::changed()` resolves when the sender sends a new
        // value.  If the sender has already sent before we call `changed()`,
        // the channel marks itself as changed and `changed()` returns
        // immediately.
        let _ = self.rx.changed().await;
        self.rx.borrow_and_update().clone()
    }
}

// ---------------------------------------------------------------------------
// InFlightEntry
// ---------------------------------------------------------------------------

struct InFlightEntry {
    tx: watch::Sender<Option<ChatCompletionResponse>>,
    created_at: Instant,
}

// ---------------------------------------------------------------------------
// RequestDeduplicator
// ---------------------------------------------------------------------------

/// Deduplicates identical concurrent requests.
///
/// When the same cache key arrives within a short window, only the first
/// request is forwarded to the provider. Subsequent requests wait for the
/// first one to complete and share its response, saving provider calls and
/// cost.
///
/// Entries are automatically cleaned up after `entry_ttl`.
pub struct RequestDeduplicator {
    map: Arc<DashMap<String, InFlightEntry>>,
    entry_ttl: Duration,
}

impl RequestDeduplicator {
    /// Create a new deduplicator. Entries older than `entry_ttl` are
    /// automatically removed on each `check_or_wait` call.
    pub fn new(entry_ttl: Duration) -> Self {
        Self {
            map: Arc::new(DashMap::new()),
            entry_ttl,
        }
    }

    /// Check if a request with the given key is already in flight.
    ///
    /// * `FirstRequest` — no in-flight request exists; the caller should
    ///   execute the request and call `complete` on the returned guard.
    /// * `Duplicate` — an identical request is in flight; the caller should
    ///   `wait` on the returned receiver.
    pub fn check_or_wait(&self, key: &str) -> DedupResult {
        // Garbage-collect stale entries first.
        self.cleanup();

        // Use the entry API for atomicity to avoid TOCTOU races.
        let map = Arc::clone(&self.map);

        match self.map.entry(key.to_string()) {
            dashmap::Entry::Occupied(occ) => {
                let entry = occ.get();
                let rx = entry.tx.subscribe();
                DedupResult::Duplicate(DedupReceiver { rx })
            }
            dashmap::Entry::Vacant(vac) => {
                let (tx, _rx) = watch::channel(None);
                let tx_clone = tx.clone();
                vac.insert(InFlightEntry {
                    tx,
                    created_at: Instant::now(),
                });
                DedupResult::FirstRequest(DedupGuard {
                    key: key.to_string(),
                    map,
                    tx: tx_clone,
                    completed: false,
                })
            }
        }
    }

    /// Notify all waiters that the first request with this key completed.
    ///
    /// This is a convenience that looks up the entry and notifies. In most
    /// cases callers use `DedupGuard::complete` instead.
    pub fn complete(&self, key: &str, response: ChatCompletionResponse) {
        if let Some(entry) = self.map.get(key) {
            let _ = entry.tx.send(Some(response));
        }
        self.map.remove(key);
    }

    /// Remove entries older than `entry_ttl`.
    fn cleanup(&self) {
        let cutoff = self.entry_ttl;
        self.map.retain(|_k, v| v.created_at.elapsed() < cutoff);
    }
}

impl Default for RequestDeduplicator {
    fn default() -> Self {
        Self::new(Duration::from_secs(2))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChatCompletionResponse, Choice, Message, Usage};

    fn dummy_response(id: &str) -> ChatCompletionResponse {
        ChatCompletionResponse {
            id: id.to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: "test-model".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: Some(serde_json::Value::String("Hello!".to_string())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            x_router_metadata: None,
        }
    }

    #[test]
    fn test_first_request_returns_guard() {
        let dedup = RequestDeduplicator::default();
        let result = dedup.check_or_wait("key-1");
        assert!(
            matches!(result, DedupResult::FirstRequest(_)),
            "First request should return FirstRequest"
        );
    }

    #[test]
    fn test_duplicate_returns_receiver() {
        let dedup = RequestDeduplicator::default();

        // First request -- grab the guard.
        let guard = match dedup.check_or_wait("key-2") {
            DedupResult::FirstRequest(g) => g,
            _ => panic!("Expected FirstRequest"),
        };

        // Second request with same key -- should be Duplicate.
        let result = dedup.check_or_wait("key-2");
        assert!(
            matches!(result, DedupResult::Duplicate(_)),
            "Same key should return Duplicate"
        );

        // Clean up.
        guard.complete(dummy_response("resp-1"));
    }

    #[tokio::test]
    async fn test_complete_broadcasts_response() {
        let dedup = RequestDeduplicator::default();

        let guard = match dedup.check_or_wait("key-3") {
            DedupResult::FirstRequest(g) => g,
            _ => panic!("Expected FirstRequest"),
        };

        let receiver = match dedup.check_or_wait("key-3") {
            DedupResult::Duplicate(r) => r,
            _ => panic!("Expected Duplicate"),
        };

        let expected = dummy_response("resp-3");
        let expected_id = expected.id.clone();

        // Complete in a separate task so the receiver can await.
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            guard.complete(expected);
        });

        let got = receiver.wait().await;
        assert!(got.is_some(), "Should receive the response");
        assert_eq!(got.unwrap().id, expected_id);
    }

    #[test]
    fn test_different_keys_are_independent() {
        let dedup = RequestDeduplicator::default();

        let _guard1 = match dedup.check_or_wait("key-a") {
            DedupResult::FirstRequest(g) => g,
            _ => panic!("Expected FirstRequest for key-a"),
        };

        let result = dedup.check_or_wait("key-b");
        assert!(
            matches!(result, DedupResult::FirstRequest(_)),
            "Different key should return FirstRequest"
        );
    }

    #[test]
    fn test_cleanup_removes_stale_entries() {
        let dedup = RequestDeduplicator::new(Duration::from_millis(1));

        // Insert and immediately drop the guard.
        {
            let _guard = match dedup.check_or_wait("stale-key") {
                DedupResult::FirstRequest(g) => g,
                _ => panic!("Expected FirstRequest"),
            };
            // Guard dropped here -- entry removed.
        }

        // After the guard is dropped the entry is removed, so a new
        // check should return FirstRequest.
        let result = dedup.check_or_wait("stale-key");
        assert!(
            matches!(result, DedupResult::FirstRequest(_)),
            "After guard drop, key should be available again"
        );
    }
}
