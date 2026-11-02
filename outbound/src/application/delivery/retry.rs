use std::time::Duration;

use chrono::{DateTime, Utc};

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
    /// `attempt` is the attempt that has just failed.
    pub fn on_failure(&self, attempt: u32) -> RetryDecision {
        if attempt >= self.max_attempts {
            return RetryDecision::GiveUp;
        }

        let delay = chrono::Duration::from_std(self.retry_delay).unwrap_or(chrono::Duration::MAX);

        RetryDecision::RetryAt(Utc::now() + delay)
    }
}
