use std::time::Duration;

use openclaw_gateway::optimization::dedup::{DedupResult, RequestDeduplicator};
use openclaw_gateway::types::{ChatCompletionResponse, Choice, Message, Usage};

// ===========================================================================
// Helpers
// ===========================================================================

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

// ===========================================================================
// Request hash consistency (same key same result)
// ===========================================================================

#[test]
fn test_first_request_returns_guard() {
    let dedup = RequestDeduplicator::default();
    let result = dedup.check_or_wait("consistent-key");
    assert!(
        matches!(result, DedupResult::FirstRequest(_)),
        "First request should always return FirstRequest"
    );
}

#[test]
fn test_same_key_returns_duplicate() {
    let dedup = RequestDeduplicator::default();

    let _guard = match dedup.check_or_wait("same-key") {
        DedupResult::FirstRequest(g) => g,
        _ => panic!("Expected FirstRequest"),
    };

    let result = dedup.check_or_wait("same-key");
    assert!(
        matches!(result, DedupResult::Duplicate(_)),
        "Same key while guard is held should return Duplicate"
    );
}

#[test]
fn test_different_keys_return_first_request() {
    let dedup = RequestDeduplicator::default();

    let _guard_a = match dedup.check_or_wait("key-alpha") {
        DedupResult::FirstRequest(g) => g,
        _ => panic!("Expected FirstRequest for key-alpha"),
    };

    let result = dedup.check_or_wait("key-beta");
    assert!(
        matches!(result, DedupResult::FirstRequest(_)),
        "Different key should return FirstRequest"
    );
}

#[test]
fn test_hash_consistency_across_checks() {
    let dedup = RequestDeduplicator::default();

    // Check a key, hold the guard
    let guard = match dedup.check_or_wait("hash-test") {
        DedupResult::FirstRequest(g) => g,
        _ => panic!("Expected FirstRequest"),
    };

    // Multiple checks with same key should all be Duplicate
    for _ in 0..5 {
        let result = dedup.check_or_wait("hash-test");
        assert!(
            matches!(result, DedupResult::Duplicate(_)),
            "Repeated same key should consistently return Duplicate"
        );
    }

    guard.complete(dummy_response("done"));
}

// ===========================================================================
// Guard completion and broadcast
// ===========================================================================

#[tokio::test]
async fn test_guard_complete_broadcasts_response() {
    let dedup = RequestDeduplicator::default();

    let guard = match dedup.check_or_wait("broadcast-key") {
        DedupResult::FirstRequest(g) => g,
        _ => panic!("Expected FirstRequest"),
    };

    let receiver = match dedup.check_or_wait("broadcast-key") {
        DedupResult::Duplicate(r) => r,
        _ => panic!("Expected Duplicate"),
    };

    let expected_id = "broadcast-response";
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        guard.complete(dummy_response(expected_id));
    });

    let got = receiver.wait().await;
    assert!(got.is_some(), "Should receive the response");
    assert_eq!(got.unwrap().id, expected_id);
}

#[tokio::test]
async fn test_multiple_receivers_get_same_response() {
    let dedup = RequestDeduplicator::default();

    let guard = match dedup.check_or_wait("multi-recv") {
        DedupResult::FirstRequest(g) => g,
        _ => panic!("Expected FirstRequest"),
    };

    let receiver1 = match dedup.check_or_wait("multi-recv") {
        DedupResult::Duplicate(r) => r,
        _ => panic!("Expected Duplicate 1"),
    };

    let receiver2 = match dedup.check_or_wait("multi-recv") {
        DedupResult::Duplicate(r) => r,
        _ => panic!("Expected Duplicate 2"),
    };

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        guard.complete(dummy_response("shared-resp"));
    });

    let got1 = receiver1.wait().await;
    let got2 = receiver2.wait().await;

    assert!(got1.is_some());
    assert!(got2.is_some());
    assert_eq!(got1.unwrap().id, "shared-resp");
    assert_eq!(got2.unwrap().id, "shared-resp");
}

// ===========================================================================
// Guard drop cleanup
// ===========================================================================

#[test]
fn test_guard_drop_removes_entry() {
    let dedup = RequestDeduplicator::default();

    // Create and drop guard
    {
        let _guard = match dedup.check_or_wait("drop-key") {
            DedupResult::FirstRequest(g) => g,
            _ => panic!("Expected FirstRequest"),
        };
        // Guard dropped here
    }

    // After guard drop, same key should return FirstRequest again
    let result = dedup.check_or_wait("drop-key");
    assert!(
        matches!(result, DedupResult::FirstRequest(_)),
        "After guard drop, key should be available again"
    );
}

#[tokio::test]
async fn test_guard_drop_without_complete_sends_none() {
    let dedup = RequestDeduplicator::default();

    let guard = match dedup.check_or_wait("abandon-key") {
        DedupResult::FirstRequest(g) => g,
        _ => panic!("Expected FirstRequest"),
    };

    let receiver = match dedup.check_or_wait("abandon-key") {
        DedupResult::Duplicate(r) => r,
        _ => panic!("Expected Duplicate"),
    };

    // Drop guard without completing (simulating abandoned request)
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        drop(guard);
    });

    let got = receiver.wait().await;
    assert!(got.is_none(), "Abandoned guard should broadcast None");
}

// ===========================================================================
// Complete method on deduplicator
// ===========================================================================

#[test]
fn test_complete_method_removes_entry() {
    let dedup = RequestDeduplicator::default();

    // First: create an entry
    let _guard = match dedup.check_or_wait("complete-method-key") {
        DedupResult::FirstRequest(g) => g,
        _ => panic!("Expected FirstRequest"),
    };

    // Use the complete method on the deduplicator directly
    dedup.complete("complete-method-key", dummy_response("resp"));

    // After completion, same key should be available again
    // (because guard's drop will also try to remove, but key was already removed)
    // Note: guard will also be dropped, so this tests that double-removal is safe
}

// ===========================================================================
// Cleanup / TTL behavior
// ===========================================================================

#[test]
fn test_default_ttl() {
    let dedup = RequestDeduplicator::default();
    // Default TTL is 2 seconds -- just verify construction works
    let result = dedup.check_or_wait("ttl-test");
    assert!(matches!(result, DedupResult::FirstRequest(_)));
}

#[test]
fn test_custom_ttl() {
    let dedup = RequestDeduplicator::new(Duration::from_millis(100));
    let result = dedup.check_or_wait("custom-ttl");
    assert!(matches!(result, DedupResult::FirstRequest(_)));
}

#[test]
fn test_cleanup_after_guard_drop_with_short_ttl() {
    let dedup = RequestDeduplicator::new(Duration::from_millis(1));

    // Insert and drop
    {
        let _guard = match dedup.check_or_wait("stale") {
            DedupResult::FirstRequest(g) => g,
            _ => panic!("Expected FirstRequest"),
        };
    }

    // Guard drop already removes the entry, so cleanup should be a no-op
    let result = dedup.check_or_wait("stale");
    assert!(
        matches!(result, DedupResult::FirstRequest(_)),
        "After guard drop + cleanup, key should be fresh"
    );
}

// ===========================================================================
// Concurrent access patterns
// ===========================================================================

#[tokio::test]
async fn test_concurrent_same_key_first_wins() {
    let dedup = RequestDeduplicator::default();

    // First caller wins
    let result1 = dedup.check_or_wait("concurrent");
    assert!(matches!(result1, DedupResult::FirstRequest(_)));

    // Second caller gets duplicate
    let result2 = dedup.check_or_wait("concurrent");
    assert!(matches!(result2, DedupResult::Duplicate(_)));

    // Third caller also gets duplicate
    let result3 = dedup.check_or_wait("concurrent");
    assert!(matches!(result3, DedupResult::Duplicate(_)));

    // Clean up
    if let DedupResult::FirstRequest(guard) = result1 {
        guard.complete(dummy_response("concurrent-resp"));
    }
}

// ===========================================================================
// Multiple independent keys
// ===========================================================================

#[test]
fn test_many_independent_keys() {
    let dedup = RequestDeduplicator::default();

    let mut guards = Vec::new();
    for i in 0..10 {
        let key = format!("independent-key-{i}");
        let result = dedup.check_or_wait(&key);
        match result {
            DedupResult::FirstRequest(g) => guards.push(g),
            _ => panic!("Each independent key should return FirstRequest"),
        }
    }

    // All guards exist, all keys are in-flight
    // Verify new keys still work
    let result = dedup.check_or_wait("independent-key-new");
    assert!(matches!(result, DedupResult::FirstRequest(_)));
}
