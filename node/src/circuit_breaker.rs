use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

/// Circuit breaker states following the standard pattern
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Too many failures - requests are blocked
    Open,
    /// Testing if system has recovered - limited requests allowed
    HalfOpen,
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before attempting recovery
    pub timeout: Duration,
    /// Number of successful requests needed to close circuit from half-open
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            success_threshold: 2,
        }
    }
}

/// Circuit breaker for emergency halt and automatic recovery
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    config: CircuitBreakerConfig,
}

struct CircuitBreakerState {
    current_state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        info!("Initializing circuit breaker with threshold: {}", config.failure_threshold);
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState {
                current_state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
            })),
            config,
        }
    }

    /// Check if request should be allowed through
    pub fn is_request_allowed(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        
        match state.current_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() >= self.config.timeout {
                        info!("Circuit breaker transitioning to HalfOpen");
                        state.current_state = CircuitState::HalfOpen;
                        state.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        
        match state.current_state {
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    info!("Circuit breaker closing after {} successes", state.success_count);
                    state.current_state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                state.failure_count = 0;
            }
            CircuitState::Open => {
                // Ignore successes when open
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        
        match state.current_state {
            CircuitState::Closed => {
                state.failure_count += 1;
                state.last_failure_time = Some(Instant::now());
                
                if state.failure_count >= self.config.failure_threshold {
                    error!("Circuit breaker opening after {} failures", state.failure_count);
                    state.current_state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker reopening after failure in HalfOpen state");
                state.current_state = CircuitState::Open;
                state.last_failure_time = Some(Instant::now());
                state.success_count = 0;
            }
            CircuitState::Open => {
                // Update last failure time
                state.last_failure_time = Some(Instant::now());
            }
        }
    }

    /// Manually open the circuit (emergency halt)
    pub fn trip(&self) {
        let mut state = self.state.lock().unwrap();
        error!("Circuit breaker manually tripped - emergency halt");
        state.current_state = CircuitState::Open;
        state.last_failure_time = Some(Instant::now());
    }

    /// Manually close the circuit (manual recovery)
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        info!("Circuit breaker manually reset");
        state.current_state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
    }

    /// Get current state
    pub fn get_state(&self) -> CircuitState {
        self.state.lock().unwrap().current_state.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            timeout: Duration::from_secs(1),
            success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert!(cb.is_request_allowed());

        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        assert!(!cb.is_request_allowed());
    }

    #[test]
    fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Wait for timeout
        thread::sleep(Duration::from_millis(150));

        // Should transition to HalfOpen
        assert!(cb.is_request_allowed());
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_closes_after_successes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Wait and transition to HalfOpen
        thread::sleep(Duration::from_millis(150));
        cb.is_request_allowed();

        // Record successes
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);
        
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    #[test]
    fn test_manual_trip_and_reset() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());

        assert_eq!(cb.get_state(), CircuitState::Closed);

        // Manual trip
        cb.trip();
        assert_eq!(cb.get_state(), CircuitState::Open);
        assert!(!cb.is_request_allowed());

        // Manual reset
        cb.reset();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert!(cb.is_request_allowed());
    }

    #[test]
    fn test_failure_in_half_open_reopens() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();

        // Wait and transition to HalfOpen
        thread::sleep(Duration::from_millis(150));
        cb.is_request_allowed();
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);

        // Failure in HalfOpen should reopen
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
    }
}
