use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxStatus {
    Pending,
    Processing,
    Processed,
    Failed,
    Quarantined,
}

impl InboxStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Processed => "processed",
            Self::Failed => "failed",
            Self::Quarantined => "quarantined",
        }
    }
}

/// A message to be written to the inbox (ingester side).
#[derive(Debug, Clone)]
pub struct NewInboxMessage {
    pub source: String,
    pub stream_name: String,
    pub queue_message_id: String,
    pub message_type: String,
    pub schema_version: Option<i32>,
    pub payload: Option<Value>,
    pub raw_payload: Option<Vec<u8>>,
    pub status: InboxStatus,
    pub last_error: Option<String>,
}

impl NewInboxMessage {
    /// A well-formed message, ready for processing.
    pub fn valid(
        source: impl Into<String>,
        stream_name: impl Into<String>,
        queue_message_id: impl Into<String>,
        message_type: impl Into<String>,
        schema_version: Option<i32>,
        payload: Value,
    ) -> Self {
        Self {
            source: source.into(),
            stream_name: stream_name.into(),
            queue_message_id: queue_message_id.into(),
            message_type: message_type.into(),
            schema_version,
            payload: Some(payload),
            raw_payload: None,
            status: InboxStatus::Pending,
            last_error: None,
        }
    }

    /// A message that could not be parsed. Stored as quarantined so it is
    /// never retried, but is kept for investigation (then ACKed).
    pub fn malformed(
        source: impl Into<String>,
        stream_name: impl Into<String>,
        queue_message_id: impl Into<String>,
        raw: Vec<u8>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            stream_name: stream_name.into(),
            queue_message_id: queue_message_id.into(),
            message_type: "unknown".into(),
            schema_version: None,
            payload: None,
            raw_payload: Some(raw),
            status: InboxStatus::Quarantined,
            last_error: Some(reason.into()),
        }
    }
}

/// A claimed inbox row (processor side).
#[derive(Debug, Clone, FromRow)]
pub struct InboxMessage {
    pub id: i64,
    pub source: String,
    pub stream_name: String,
    pub queue_message_id: String,
    pub message_type: String,
    pub schema_version: Option<i32>,
    pub payload: Option<Value>,
    pub attempts: i32,
    pub deferrals: i32,
    pub received_at: DateTime<Utc>,
}
