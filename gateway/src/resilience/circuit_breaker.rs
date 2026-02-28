use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU8, Ordering};

use dashmap::DashMap;

// ---------------------------------------------------------------------------
// CircuitBreakerState
// ---------------------------------------------------------------------------

/// The three states of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Normal operation — requests flow through.
    Closed = 0,
    /// Too many failures — requests are blocked.
    Open = 1,
    /// Cooldown expired — one probe request is allowed.
    HalfOpen = 2,
}

impl From<u8> for CircuitBreakerState {
    fn from(v: u8) -> Self {
        match v {
            1 => CircuitBreakerState::Open,
            2 => CircuitBreakerState::HalfOpen,
            _ => CircuitBreakerState::Closed,
        }
    }
}

// ---------------------------------------------------------------------------
// CircuitBreaker
// ---------------------------------------------------------------------------

/// Lock-free circuit breaker that tracks provider health.
///
/// Uses atomics so it can be shared across threads without a `Mutex`.
pub struct CircuitBreaker {
    /// Raw state stored as `u8` — see [`CircuitBreakerState`].
    state: AtomicU8,
    /// Running count of consecutive failures.
    failure_count: AtomicU32,
    /// Unix-epoch seconds of the most recent failure.
    last_failure: AtomicI64,
    /// Seconds to wait before transitioning Open -> HalfOpen.
    cooldown_secs: u64,
    /// Number of consecutive failures required to open the circuit.
    threshold: u32,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    ///
    /// * `threshold` — consecutive failures before opening (default 5).
    /// * `cooldown_secs` — seconds to wait in Open before transitioning to
    ///   HalfOpen (default 30).
    pub fn new(threshold: u32, cooldown_secs: u64) -> Self {
        Self {
            state: AtomicU8::new(CircuitBreakerState::Closed as u8),
            failure_count: AtomicU32::new(0),
            last_failure: AtomicI64::new(0),
            cooldown_secs,
            threshold,
        }
    }

    /// Returns `true` if the circuit breaker allows a request to proceed.
    ///
    /// * **Closed** — always allows.
    /// * **HalfOpen** — allows (probe request).
    /// * **Open** — blocks.
    pub fn can_execute(&self) -> bool {
        match self.state() {
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => true,
            CircuitBreakerState::Open => false,
        }
    }

    /// Record a successful request — resets counters and closes the circuit.
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.state
            .store(CircuitBreakerState::Closed as u8, Ordering::SeqCst);
    }

    /// Record a failed request — increments failure counter and, if the
    /// threshold is reached, opens the circuit.
    pub fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        let now = chrono::Utc::now().timestamp();
        self.last_failure.store(now, Ordering::SeqCst);

        if count >= self.threshold {
            self.state
                .store(CircuitBreakerState::Open as u8, Ordering::SeqCst);
        }
    }

    /// Returns the current logical state, promoting Open -> HalfOpen when the
    /// cooldown has elapsed.
    pub fn state(&self) -> CircuitBreakerState {
        let raw: CircuitBreakerState = self.state.load(Ordering::SeqCst).into();

        if raw == CircuitBreakerState::Open {
            let last = self.last_failure.load(Ordering::SeqCst);
            let now = chrono::Utc::now().timestamp();

            if now - last >= self.cooldown_secs as i64 {
                // Attempt to transition to HalfOpen. CAS avoids racing with
                // another thread that may have already promoted or re-opened.
                let _ = self.state.compare_exchange(
                    CircuitBreakerState::Open as u8,
                    CircuitBreakerState::HalfOpen as u8,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                return CircuitBreakerState::HalfOpen;
            }
        }

        raw
    }

    /// Read the failure count (mainly useful for testing / diagnostics).
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, 30)
    }
}

// ---------------------------------------------------------------------------
// ProviderCircuitBreakers
// ---------------------------------------------------------------------------

/// Thread-safe registry that maintains one [`CircuitBreaker`] per provider.
pub struct ProviderCircuitBreakers {
    breakers: DashMap<String, CircuitBreaker>,
    default_threshold: u32,
    default_cooldown_secs: u64,
}

impl ProviderCircuitBreakers {
    /// Create a registry with the given default threshold and cooldown.
    pub fn new(default_threshold: u32, default_cooldown_secs: u64) -> Self {
        Self {
            breakers: DashMap::new(),
            default_threshold,
            default_cooldown_secs,
        }
    }

    /// Get the circuit breaker for `provider_name`, creating one with default
    /// settings if it does not exist yet.
    ///
    /// Returns a `dashmap::mapref::one::Ref` that derefs to `CircuitBreaker`.
    pub fn get_or_create(
        &self,
        provider_name: &str,
    ) -> dashmap::mapref::one::Ref<'_, String, CircuitBreaker> {
        // Insert only if absent.
        self.breakers
            .entry(provider_name.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.default_threshold, self.default_cooldown_secs));

        self.breakers.get(provider_name).expect("just inserted")
    }
}

impl Default for ProviderCircuitBreakers {
    fn default() -> Self {
        Self::new(5, 30)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_in_closed_state() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn opens_after_threshold_failures() {
        let cb = CircuitBreaker::new(3, 30);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        cb.record_failure(); // 3rd failure — meets threshold
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn blocks_requests_when_open() {
        let cb = CircuitBreaker::new(1, 9999);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn transitions_to_half_open_after_cooldown() {
        // Use a long cooldown so the breaker stays Open initially.
        let cb = CircuitBreaker::new(1, 9999);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.can_execute());

        // Now simulate cooldown expiry by back-dating `last_failure`.
        let past = chrono::Utc::now().timestamp() - 10_000;
        cb.last_failure.store(past, Ordering::SeqCst);

        // The next state() call should transition Open -> HalfOpen.
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
        assert!(cb.can_execute());
    }

    #[test]
    fn resets_to_closed_on_success() {
        let cb = CircuitBreaker::new(1, 9999);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // Simulate cooldown expiry to reach HalfOpen.
        let past = chrono::Utc::now().timestamp() - 10_000;
        cb.last_failure.store(past, Ordering::SeqCst);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn provider_circuit_breakers_creates_per_provider() {
        let pcb = ProviderCircuitBreakers::default();
        {
            let cb = pcb.get_or_create("openai");
            assert_eq!(cb.state(), CircuitBreakerState::Closed);
            cb.record_failure();
        }

        // Different provider should be independent.
        let cb2 = pcb.get_or_create("anthropic");
        assert_eq!(cb2.state(), CircuitBreakerState::Closed);
        assert_eq!(cb2.failure_count(), 0);

        // Same provider should share state.
        let cb3 = pcb.get_or_create("openai");
        assert_eq!(cb3.failure_count(), 1);
    }
}
