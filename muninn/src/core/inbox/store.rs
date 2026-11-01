use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::model::{InboxMessage, NewInboxMessage};
use crate::error::Result;

/// Inbox operations that do not need to share a transaction with business
/// tables. The one that does, "mark processed", is
/// `postgres::mark_processed_tx`, called by the processor inside the
/// projector's transaction.
#[async_trait]
pub trait InboxStore: Send + Sync {
    /// Insert in one transaction, skipping duplicates
    /// (`stream_name`, `queue_message_id`). Returns the number of new rows.
    /// After this returns Ok, it is safe to ACK every message in the batch,
    /// including the duplicates.
    async fn insert_batch(&self, messages: &[NewInboxMessage]) -> Result<u64>;

    /// Atomically claim up to `limit` due pending rows for `source`.
    async fn claim_batch(
        &self,
        source: &str,
        worker_id: &str,
        limit: i64,
        lease: Duration,
    ) -> Result<Vec<InboxMessage>>;

    /// Failure that counts against the retry budget.
    async fn schedule_retry(&self, id: i64, error: &str, available_at: DateTime<Utc>)
        -> Result<()>;

    /// Waiting on a dependency (e.g. event not persisted yet). Does NOT
    /// count against `attempts`.
    async fn defer(&self, id: i64, reason: &str, available_at: DateTime<Utc>) -> Result<()>;

    /// Retry budget exhausted.
    async fn mark_failed(&self, id: i64, error: &str) -> Result<()>;

    /// Permanently invalid (bad schema, bad business data).
    async fn mark_quarantined(&self, id: i64, error: &str) -> Result<()>;

    /// Return expired leases to `pending`, or to `failed` once they exceed
    /// `max_attempts` (protects against messages that crash the worker).
    /// Returns the number of rows touched.
    async fn reap_stale(&self, max_attempts: i32) -> Result<u64>;
}