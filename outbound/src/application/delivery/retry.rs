use std::time::Duration;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Real processing failures allowed before the row becomes `failed`.
    pub max_attempts: i32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_factor: u32,

    /// Waiting on a dependency (e.g. the event isn't stored yet).
    /// Tracked separately, with a much larger budget.
    pub defer_delay: Duration,
    pub max_defer_delay: Duration,
    pub max_deferrals: i32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            initial_backoff: Duration::from_secs(5),
            max_backoff: Duration::from_secs(300),
            backoff_factor: 3, // 5s, 15s, 45s, 135s, 300s, 300s...
            defer_delay: Duration::from_secs(5),
            max_defer_delay: Duration::from_secs(60),
            max_deferrals: 2000, // roughly 33h at the 60s cap
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    RetryAt(DateTime<Utc>),
    GiveUp,
}

impl RetryPolicy {
    /// `attempts` is the number of failures already recorded
    /// (`InboxMessage::attempts` at claim time). This failure is one more.
    pub fn on_failure(&self, attempts: i32) -> RetryDecision {
        if attempts.saturating_add(1) >= self.max_attempts {
            return RetryDecision::GiveUp;
        }
        let delay = exp_delay(
            self.initial_backoff,
            self.backoff_factor,
            attempts.max(0) as u32,
            self.max_backoff,
        );
        RetryDecision::RetryAt(after(delay))
    }

    /// `deferrals` is the count already recorded (`InboxMessage::deferrals`).
    pub fn on_defer(&self, deferrals: i32) -> RetryDecision {
        if deferrals.saturating_add(1) >= self.max_deferrals {
            return RetryDecision::GiveUp;
        }
        let delay = exp_delay(
            self.defer_delay,
            2,
            deferrals.max(0) as u32,
            self.max_defer_delay,
        );
        RetryDecision::RetryAt(after(delay))
    }
}

fn exp_delay(base: Duration, factor: u32, n: u32, cap: Duration) -> Duration {
    let mult = (factor as u64).checked_pow(n).unwrap_or(u64::MAX);
    // f64 avoids integer overflow; the cap keeps it finite and sane.
    let secs = (base.as_secs_f64() * mult as f64).min(cap.as_secs_f64());
    Duration::from_secs_f64(secs)
}

fn after(delay: Duration) -> DateTime<Utc> {
    let d = chrono::Duration::from_std(delay).unwrap_or_else(|_| chrono::Duration::seconds(300));
    Utc::now() + d
}
