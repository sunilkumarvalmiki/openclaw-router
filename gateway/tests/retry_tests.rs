use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use openclaw_gateway::error::AppError;
use openclaw_gateway::resilience::retry::is_retriable;
use openclaw_gateway::resilience::{RetryConfig, with_retry};

// ===========================================================================
// is_retriable() tests
// ===========================================================================

#[test]
fn test_retriable_429_rate_limit() {
    assert!(is_retriable(&AppError::ProviderError("429 Too Many Requests".into())));
}

#[test]
fn test_retriable_500_server_error() {
    assert!(is_retriable(&AppError::ProviderError("status 500".into())));
}

#[test]
fn test_retriable_502_bad_gateway() {
    assert!(is_retriable(&AppError::ProviderError("502 Bad Gateway".into())));
}

#[test]
fn test_retriable_503_unavailable() {
    assert!(is_retriable(&AppError::ProviderError("503 Service Unavailable".into())));
}

#[test]
fn test_retriable_504_timeout() {
    assert!(is_retriable(&AppError::ProviderError("504 Gateway Timeout".into())));
}

#[test]
fn test_retriable_timeout_message() {
    assert!(is_retriable(&AppError::ProviderError("request timed out".into())));
}

#[test]
fn test_retriable_timeout_uppercase() {
    // is_retriable converts to lowercase, so uppercase "TIMEOUT" should also match
    assert!(is_retriable(&AppError::ProviderError("Connection TIMEOUT occurred".into())));
}

#[test]
fn test_not_retriable_400_bad_request() {
    assert!(!is_retriable(&AppError::ProviderError("400 Bad Request".into())));
}

#[test]
fn test_not_retriable_401_unauthorized() {
    assert!(!is_retriable(&AppError::ProviderError("401 Unauthorized".into())));
}

#[test]
fn test_not_retriable_403_forbidden() {
    assert!(!is_retriable(&AppError::ProviderError("403 Forbidden".into())));
}

#[test]
fn test_not_retriable_404_not_found() {
    assert!(!is_retriable(&AppError::ProviderError("404 Not Found".into())));
}

#[test]
fn test_not_retriable_422_unprocessable() {
    assert!(!is_retriable(&AppError::ProviderError("422 Unprocessable Entity".into())));
}

#[test]
fn test_not_retriable_bad_request_variant() {
    assert!(!is_retriable(&AppError::BadRequest("missing field".into())));
}

#[test]
fn test_not_retriable_unauthorized_variant() {
    assert!(!is_retriable(&AppError::Unauthorized("invalid key".into())));
}

#[test]
fn test_not_retriable_not_found_variant() {
    assert!(!is_retriable(&AppError::NotFound("model not found".into())));
}

#[test]
fn test_not_retriable_internal_variant() {
    assert!(!is_retriable(&AppError::Internal("panic".into())));
}

#[test]
fn test_not_retriable_cache_error() {
    assert!(!is_retriable(&AppError::CacheError("redis down".into())));
}

#[test]
fn test_not_retriable_database_error() {
    assert!(!is_retriable(&AppError::DatabaseError("connection lost".into())));
}

#[test]
fn test_not_retriable_rate_limit_variant() {
    // The AppError::RateLimit variant (not ProviderError) should NOT be retriable
    assert!(!is_retriable(&AppError::RateLimit("too many requests".into())));
}

#[test]
fn test_retriable_mixed_case_status_code() {
    // The is_retriable function uses lowercase, so mixed-case messages with codes should match
    assert!(is_retriable(&AppError::ProviderError("Error: 502 bad gateway from upstream".into())));
}

#[test]
fn test_retriable_provider_error_with_timed_out() {
    assert!(is_retriable(&AppError::ProviderError("Provider request timed out: connection error".into())));
}

// ===========================================================================
// RetryConfig tests
// ===========================================================================

#[test]
fn test_retry_config_default() {
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.base_delay_ms, 200);
    assert_eq!(config.max_delay_ms, 5000);
}

#[test]
fn test_retry_config_custom() {
    let config = RetryConfig {
        max_retries: 5,
        base_delay_ms: 100,
        max_delay_ms: 3000,
    };
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.base_delay_ms, 100);
    assert_eq!(config.max_delay_ms, 3000);
}

#[test]
fn test_retry_config_zero_retries() {
    let config = RetryConfig {
        max_retries: 0,
        base_delay_ms: 100,
        max_delay_ms: 1000,
    };
    assert_eq!(config.max_retries, 0);
}

// ===========================================================================
// with_retry() async tests
// ===========================================================================

#[tokio::test]
async fn test_with_retry_immediate_success() {
    let config = RetryConfig {
        max_retries: 3,
        base_delay_ms: 10,
        max_delay_ms: 100,
    };

    let result = with_retry(&config, || async { Ok::<&str, AppError>("success") }).await;
    assert_eq!(result.unwrap(), "success");
}

#[tokio::test]
async fn test_with_retry_succeeds_after_two_failures() {
    let config = RetryConfig {
        max_retries: 3,
        base_delay_ms: 10,
        max_delay_ms: 50,
    };

    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();

    let result = with_retry(&config, || {
        let c = c.clone();
        async move {
            let n = c.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                Err(AppError::ProviderError("502 Bad Gateway".into()))
            } else {
                Ok("recovered")
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), "recovered");
    assert_eq!(counter.load(Ordering::SeqCst), 3); // 1 initial + 2 retries
}

#[tokio::test]
async fn test_with_retry_exhausts_all_retries() {
    let config = RetryConfig {
        max_retries: 2,
        base_delay_ms: 10,
        max_delay_ms: 50,
    };

    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();

    let result = with_retry(&config, || {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Err::<(), AppError>(AppError::ProviderError("503 Unavailable".into()))
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 3); // 1 initial + 2 retries
}

#[tokio::test]
async fn test_with_retry_no_retry_on_non_retriable() {
    let config = RetryConfig {
        max_retries: 5,
        base_delay_ms: 10,
        max_delay_ms: 50,
    };

    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();

    let result = with_retry(&config, || {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Err::<(), AppError>(AppError::BadRequest("invalid input".into()))
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1); // No retries at all
}

#[tokio::test]
async fn test_with_retry_zero_max_retries() {
    let config = RetryConfig {
        max_retries: 0,
        base_delay_ms: 10,
        max_delay_ms: 50,
    };

    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();

    let result = with_retry(&config, || {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Err::<(), AppError>(AppError::ProviderError("502 Bad Gateway".into()))
        }
    })
    .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1); // Only the initial attempt
}

#[tokio::test]
async fn test_with_retry_succeeds_on_last_attempt() {
    let config = RetryConfig {
        max_retries: 3,
        base_delay_ms: 10,
        max_delay_ms: 50,
    };

    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();

    let result = with_retry(&config, || {
        let c = c.clone();
        async move {
            let n = c.fetch_add(1, Ordering::SeqCst);
            if n < 3 {
                // fail first 3 attempts (initial + 2 retries)
                Err(AppError::ProviderError("429 rate limited".into()))
            } else {
                // succeed on the 4th (last) attempt
                Ok("just in time")
            }
        }
    })
    .await;

    assert_eq!(result.unwrap(), "just in time");
    assert_eq!(counter.load(Ordering::SeqCst), 4); // 1 initial + 3 retries
}

#[tokio::test]
async fn test_with_retry_preserves_error_message() {
    let config = RetryConfig {
        max_retries: 0,
        base_delay_ms: 10,
        max_delay_ms: 50,
    };

    let result: Result<(), AppError> = with_retry(&config, || async {
        Err(AppError::BadRequest("specific error message".into()))
    })
    .await;

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("specific error message"),
        "Error message should be preserved: {}",
        err
    );
}
