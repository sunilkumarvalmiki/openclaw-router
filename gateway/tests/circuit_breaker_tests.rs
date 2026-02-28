use openclaw_gateway::resilience::{CircuitBreaker, CircuitBreakerState, ProviderCircuitBreakers};

#[test]
fn test_starts_in_closed_state() {
    let cb = CircuitBreaker::default();
    assert_eq!(cb.state(), CircuitBreakerState::Closed);
    assert!(cb.can_execute());
}

#[test]
fn test_opens_after_threshold_failures() {
    let cb = CircuitBreaker::new(5, 30);

    for _ in 0..4 {
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    cb.record_failure(); // 5th failure — meets threshold
    assert_eq!(cb.state(), CircuitBreakerState::Open);
    assert!(!cb.can_execute());
}

#[test]
fn test_blocks_requests_when_open() {
    let cb = CircuitBreaker::new(2, 9999);
    cb.record_failure();
    cb.record_failure();

    assert_eq!(cb.state(), CircuitBreakerState::Open);
    assert!(!cb.can_execute());
}

#[test]
fn test_transitions_to_half_open_after_cooldown() {
    // cooldown = 0 makes the state transition happen immediately.
    let cb = CircuitBreaker::new(1, 0);
    cb.record_failure();

    // Because cooldown = 0 seconds, reading state() transitions Open -> HalfOpen.
    assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    assert!(cb.can_execute());
}

#[test]
fn test_resets_to_closed_on_success() {
    let cb = CircuitBreaker::new(1, 0);
    cb.record_failure();

    // HalfOpen due to instant cooldown.
    assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);

    cb.record_success();
    assert_eq!(cb.state(), CircuitBreakerState::Closed);
    assert_eq!(cb.failure_count(), 0);
    assert!(cb.can_execute());
}

#[test]
fn test_provider_circuit_breakers_isolation() {
    let pcb = ProviderCircuitBreakers::default();

    // Fail openai
    {
        let cb = pcb.get_or_create("openai");
        cb.record_failure();
        cb.record_failure();
    }

    // Anthropic is unaffected
    {
        let cb = pcb.get_or_create("anthropic");
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    // OpenAI retains its count
    {
        let cb = pcb.get_or_create("openai");
        assert_eq!(cb.failure_count(), 2);
    }
}
