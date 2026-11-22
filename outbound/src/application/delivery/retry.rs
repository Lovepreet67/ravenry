use std::time::Duration;

use chrono::{DateTime, Utc};
use rand::RngExt;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Total attempts, including the initial attempt.
    pub max_attempts: u32,

    /// Fixed delay between failed attempts.
    pub retry_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            retry_delay: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    RetryAt(DateTime<Utc>),
    GiveUp,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32, retry_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            retry_delay: Duration::from_millis(retry_delay_ms),
        }
    }
    /// `attempt` is the attempt that has just failed.
    pub fn on_failure(&self, attempt: u32) -> RetryDecision {
        if attempt >= self.max_attempts {
            return RetryDecision::GiveUp;
        }
        let exp_exponent = attempt.saturating_sub(1);
        let multiplier = 2_u32.saturating_pow(exp_exponent);

        let base_backoff = self
            .retry_delay
            .saturating_mul(multiplier)
            .min(self.retry_delay * 100);

        // Full jitter: pick a random duration in [0, base_backoff]
        let jitter_max_ms = base_backoff.as_nanos() / 4;
        let jitter_ms = if jitter_max_ms > 0 {
            rand::rng()
                .random_range(0..=jitter_max_ms)
                .min(u64::MAX as u128) as u64
        } else {
            0
        };
        let jitter_delay = Duration::from_nanos(jitter_ms);

        let delay = chrono::Duration::from_std(base_backoff.saturating_add(jitter_delay))
            .unwrap_or(chrono::Duration::MAX);

        RetryDecision::RetryAt(Utc::now() + delay)
    }
}
