use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgConnection;

use super::inbox::{InboxMessage, NewInboxMessage};
use super::source::StreamSource;

/// Why a projection failed. The processor maps each variant to an inbox
/// action:
///
/// | Variant     | Inbox action                              |
/// |-------------|-------------------------------------------|
/// | `Transient` | `schedule_retry` (counts toward attempts) |
/// | `Defer`     | `defer` (own budget, not attempts)        |
/// | `Invalid`   | `mark_quarantined` (never retried)        |
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("transient: {0}")]
    Transient(String),

    #[error("dependency not ready: {0}")]
    Defer(String),

    #[error("invalid message: {0}")]
    Invalid(String),
}

// Unknown database errors are retried. A projector that can recognise a
// specific error (e.g. an FK violation meaning "event not stored yet")
// should map it to `Defer` itself.
impl From<sqlx::Error> for ProjectError {
    fn from(e: sqlx::Error) -> Self {
        Self::Transient(e.to_string())
    }
}

// A payload that can't be deserialized will never succeed on retry.
impl From<serde_json::Error> for ProjectError {
    fn from(e: serde_json::Error) -> Self {
        Self::Invalid(e.to_string())
    }
}

impl From<crate::error::Error> for ProjectError {
    fn from(e: crate::error::Error) -> Self {
        Self::Transient(e.to_string())
    }
}

/// Writes one inbox message into the pipeline's business tables.
///
/// The processor owns the transaction. `conn` is already inside it, and
/// the processor marks the inbox row processed in the same transaction.
/// Return `Ok` only when the business write is complete.
///
/// Implementations MUST be idempotent (`ON CONFLICT DO NOTHING`, etc.),
/// because a message can be projected twice after a lease expiry.
#[async_trait]
pub trait Projector: Send + Sync {
    async fn project(&self, conn: &mut PgConnection, msg: &InboxMessage)
        -> Result<(), ProjectError>;
}

/// One workload (events, deliveries, or any future stream sync).
pub trait Pipeline: Send + Sync {
    /// Unique name. Also stored as `inbox_messages.source` and used as a
    /// metrics label.
    fn name(&self) -> &'static str;

    /// Consumer group shared by all instances, e.g. "persistence-events".
    fn consumer_group(&self) -> String;

    /// Which streams to consume.
    fn source(&self) -> Arc<dyn StreamSource>;

    /// Business-table writer for this pipeline.
    fn projector(&self) -> Arc<dyn Projector>;

    /// Validate a raw queue message and turn it into an inbox row.
    ///
    /// Never fails. Undecodable input or an unsupported schema version
    /// returns `NewInboxMessage::malformed(...)`, so it is stored as
    /// quarantined and the queue message can still be ACKed.
    fn prepare(&self, stream_name: &str, queue_message_id: &str, raw: &[u8]) -> NewInboxMessage;
}