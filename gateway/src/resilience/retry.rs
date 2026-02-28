use std::future::Future;

use rand::Rng;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// RetryConfig
// ---------------------------------------------------------------------------

/// Configuration for exponential-backoff retries.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (does **not** include the initial try).
    pub max_retries: u32,
    /// Base delay in milliseconds before the first retry.
    pub base_delay_ms: u64,
    /// Maximum delay cap in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 200,
            max_delay_ms: 5000,
        }
    }
}

// ---------------------------------------------------------------------------
// is_retriable
// ---------------------------------------------------------------------------

/// Decide whether an error is transient and therefore worth retrying.
///
/// Retriable errors:
/// * `ProviderError` whose message contains a `429` status (rate-limit) or
///   a 5xx status code (`500`, `502`, `503`, `504`).
///
/// Non-retriable errors:
/// * `BadRequest`, `Unauthorized`, `NotFound`, and everything else.
pub fn is_retriable(error: &AppError) -> bool {
    match error {
        AppError::ProviderError(msg) => {
            let m = msg.to_lowercase();
            m.contains("429")
                || m.contains("500")
                || m.contains("502")
                || m.contains("503")
                || m.contains("504")
                || m.contains("timeout")
                || m.contains("timed out")
        }
        // Internal / CacheError / DatabaseError could arguably be retried,
        // but we keep this strict for now.
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// with_retry
// ---------------------------------------------------------------------------

/// Execute `operation` with exponential-backoff retries.
///
/// The operation is attempted once; on retriable failure it is retried up to
/// `config.max_retries` additional times.  The delay between retries is:
///
/// ```text
/// delay = min(base_delay_ms * 2^attempt + jitter, max_delay_ms)
/// ```
///
/// where `jitter` is a random value in `[0, base_delay_ms)`.
pub async fn with_retry<F, Fut, T>(config: &RetryConfig, operation: F) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut last_error: Option<AppError> = None;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt == config.max_retries || !is_retriable(&err) {
                    return Err(err);
                }

                let delay = compute_delay(config, attempt);
                tracing::warn!(
                    attempt = attempt + 1,
                    max_retries = config.max_retries,
                    delay_ms = delay,
                    error = %err,
                    "Retrying after transient error"
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                last_error = Some(err);
            }
        }
    }

    // Should not be reached, but just in case:
    Err(last_error.unwrap_or_else(|| AppError::Internal("Retry loop exhausted".into())))
}

/// Compute the delay for a given attempt (0-based).
fn compute_delay(config: &RetryConfig, attempt: u32) -> u64 {
    let exp_delay = config.base_delay_ms.saturating_mul(1u64 << attempt);
    let jitter = rand::thread_rng().gen_range(0..config.base_delay_ms.max(1));
    let total = exp_delay.saturating_add(jitter);
    total.min(config.max_delay_ms)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_is_retriable() {
        assert!(is_retriable(&AppError::ProviderError("429 Too Many Requests".into())));
        assert!(is_retriable(&AppError::ProviderError("status 500".into())));
        assert!(is_retriable(&AppError::ProviderError("502 Bad Gateway".into())));
        assert!(is_retriable(&AppError::ProviderError("503 Unavailable".into())));
        assert!(is_retriable(&AppError::ProviderError("504 Timeout".into())));
        assert!(is_retriable(&AppError::ProviderError("request timed out".into())));

        // Non-retriable
        assert!(!is_retriable(&AppError::BadRequest("bad".into())));
        assert!(!is_retriable(&AppError::Unauthorized("nope".into())));
        assert!(!is_retriable(&AppError::NotFound("gone".into())));
        assert!(!is_retriable(&AppError::ProviderError("400 Bad Request".into())));
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_first_try() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 10,
            max_delay_ms: 100,
        };

        let result = with_retry(&config, || async { Ok::<&str, AppError>("hello") }).await;
        assert_eq!(result.unwrap(), "hello");
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_after_failures() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 10,
            max_delay_ms: 100,
        };

        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();

        let result = with_retry(&config, || {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(AppError::ProviderError("503 Unavailable".into()))
                } else {
                    Ok("recovered")
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), "recovered");
        assert_eq!(counter.load(Ordering::SeqCst), 3); // initial + 2 retries
    }

    #[tokio::test]
    async fn test_with_retry_does_not_retry_non_retriable() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 10,
            max_delay_ms: 100,
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
        assert_eq!(counter.load(Ordering::SeqCst), 1); // no retries
    }

    #[tokio::test]
    async fn test_with_retry_exhausts_retries() {
        let config = RetryConfig {
            max_retries: 2,
            base_delay_ms: 10,
            max_delay_ms: 100,
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
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 1 initial + 2 retries
    }

    #[test]
    fn test_compute_delay_respects_max() {
        let config = RetryConfig {
            max_retries: 10,
            base_delay_ms: 200,
            max_delay_ms: 5000,
        };

        for attempt in 0..10 {
            let delay = compute_delay(&config, attempt);
            assert!(delay <= config.max_delay_ms, "delay {delay} exceeds max");
        }
    }
}
