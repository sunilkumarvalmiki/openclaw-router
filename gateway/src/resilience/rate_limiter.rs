use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};

use dashmap::DashMap;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// RateLimitBucket
// ---------------------------------------------------------------------------

/// Per-key bucket holding remaining counters for a 60-second window.
pub struct RateLimitBucket {
    /// Remaining requests in the current window.
    requests_remaining: AtomicU32,
    /// Remaining tokens in the current window.
    tokens_remaining: AtomicU32,
    /// Unix-epoch seconds when the current window started.
    window_start: AtomicI64,
    /// Requests-per-minute limit.
    rpm_limit: u32,
    /// Tokens-per-minute limit.
    tpm_limit: u32,
}

impl RateLimitBucket {
    fn new(rpm: u32, tpm: u32) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            requests_remaining: AtomicU32::new(rpm),
            tokens_remaining: AtomicU32::new(tpm),
            window_start: AtomicI64::new(now),
            rpm_limit: rpm,
            tpm_limit: tpm,
        }
    }

    /// Reset the counters if the window has expired (>= 60 s since start).
    /// Returns `true` if the window was actually reset.
    fn maybe_reset_window(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        let start = self.window_start.load(Ordering::SeqCst);

        if now - start >= 60 {
            self.window_start.store(now, Ordering::SeqCst);
            self.requests_remaining
                .store(self.rpm_limit, Ordering::SeqCst);
            self.tokens_remaining
                .store(self.tpm_limit, Ordering::SeqCst);
            return true;
        }

        false
    }

    /// Try to consume one request and `estimated_tokens` tokens.
    /// Returns `Ok(())` on success or an appropriate `AppError::RateLimit`.
    fn try_consume(&self, estimated_tokens: u32) -> Result<(), AppError> {
        // Check RPM
        let req_rem = self.requests_remaining.load(Ordering::SeqCst);
        if req_rem == 0 {
            return Err(AppError::RateLimit(
                "Requests per minute limit exceeded".into(),
            ));
        }

        // Check TPM
        let tok_rem = self.tokens_remaining.load(Ordering::SeqCst);
        if tok_rem < estimated_tokens {
            return Err(AppError::RateLimit(
                "Tokens per minute limit exceeded".into(),
            ));
        }

        // Decrement counters
        self.requests_remaining.fetch_sub(1, Ordering::SeqCst);
        self.tokens_remaining
            .fetch_sub(estimated_tokens, Ordering::SeqCst);

        Ok(())
    }

    /// Adjust the token counter after actual usage is known.
    ///
    /// If the real usage differs from the estimate we already deducted, we
    /// apply the delta.  Because we already subtracted `estimated` tokens,
    /// callers should pass only the *actual* number here; the bucket will
    /// subtract any additional tokens or return unused ones.
    fn record_actual_usage(&self, actual_tokens: u32, estimated_tokens: u32) {
        if actual_tokens > estimated_tokens {
            let extra = actual_tokens - estimated_tokens;
            self.tokens_remaining.fetch_sub(extra, Ordering::SeqCst);
        } else if estimated_tokens > actual_tokens {
            let refund = estimated_tokens - actual_tokens;
            self.tokens_remaining.fetch_add(refund, Ordering::SeqCst);
        }
    }
}

// ---------------------------------------------------------------------------
// RateLimiter
// ---------------------------------------------------------------------------

/// In-memory, token-aware rate limiter keyed by API key.
///
/// Each key gets its own [`RateLimitBucket`] with independent request and
/// token counters that reset every 60 seconds.
pub struct RateLimiter {
    buckets: DashMap<String, RateLimitBucket>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: DashMap::new(),
        }
    }

    /// Check whether `api_key` may issue a request consuming
    /// `estimated_tokens`.
    ///
    /// If the limits allow, the counters are decremented and `Ok(())` is
    /// returned.  Otherwise an `AppError::RateLimit` is returned.
    ///
    /// * `rpm` / `tpm` — per-key limits used when creating a new bucket.
    pub fn check_rate_limit(
        &self,
        api_key: &str,
        estimated_tokens: u32,
        rpm: u32,
        tpm: u32,
    ) -> Result<(), AppError> {
        self.buckets
            .entry(api_key.to_string())
            .or_insert_with(|| RateLimitBucket::new(rpm, tpm));

        let bucket = self.buckets.get(api_key).expect("just inserted");

        // Reset window if expired.
        bucket.maybe_reset_window();

        // Try to consume.
        bucket.try_consume(estimated_tokens)
    }

    /// Record the actual token usage after a response is received.
    ///
    /// This adjusts the token counter so that subsequent requests see
    /// accurate remaining capacity.
    pub fn record_usage(&self, api_key: &str, actual_tokens: u32, estimated_tokens: u32) {
        if let Some(bucket) = self.buckets.get(api_key) {
            bucket.record_actual_usage(actual_tokens, estimated_tokens);
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_requests_within_limits() {
        let rl = RateLimiter::new();
        assert!(rl.check_rate_limit("key-1", 100, 10, 1000).is_ok());
        assert!(rl.check_rate_limit("key-1", 100, 10, 1000).is_ok());
    }

    #[test]
    fn blocks_requests_over_rpm_limit() {
        let rl = RateLimiter::new();

        // RPM = 2
        assert!(rl.check_rate_limit("key-rpm", 10, 2, 10000).is_ok());
        assert!(rl.check_rate_limit("key-rpm", 10, 2, 10000).is_ok());

        // 3rd request should be blocked.
        let result = rl.check_rate_limit("key-rpm", 10, 2, 10000);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::RateLimit(_)),
            "Expected RateLimit error, got: {err:?}"
        );
    }

    #[test]
    fn blocks_requests_over_token_limit() {
        let rl = RateLimiter::new();

        // TPM = 100
        assert!(rl.check_rate_limit("key-tpm", 60, 100, 100).is_ok());

        // Only 40 tokens remain, requesting 50 should fail.
        let result = rl.check_rate_limit("key-tpm", 50, 100, 100);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::RateLimit(_)),
            "Expected RateLimit error, got: {err:?}"
        );
    }

    #[test]
    fn resets_after_window_expires() {
        let rl = RateLimiter::new();

        // Exhaust RPM
        assert!(rl.check_rate_limit("key-reset", 10, 1, 10000).is_ok());
        assert!(rl.check_rate_limit("key-reset", 10, 1, 10000).is_err());

        // Manually roll the window_start back so the window looks expired.
        if let Some(bucket) = rl.buckets.get("key-reset") {
            let old = bucket.window_start.load(Ordering::SeqCst);
            bucket.window_start.store(old - 61, Ordering::SeqCst);
        }

        // After window expiry the limits should be reset.
        assert!(rl.check_rate_limit("key-reset", 10, 1, 10000).is_ok());
    }

    #[test]
    fn record_usage_adjusts_tokens() {
        let rl = RateLimiter::new();

        // estimated = 100 tokens
        assert!(rl.check_rate_limit("key-usage", 100, 10, 1000).is_ok());
        // 900 tokens remain

        // Actual usage was only 50 — we should get 50 back.
        rl.record_usage("key-usage", 50, 100);
        // 950 tokens remaining now

        // We should be able to consume 950 tokens.
        assert!(rl.check_rate_limit("key-usage", 950, 10, 1000).is_ok());
    }
}
