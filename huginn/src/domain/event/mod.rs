use chrono::{DateTime, Utc};
use uuid::Uuid;

/// List item. Deliberately has no payload.
#[derive(Debug, Clone)]
pub struct EventSummary {
    pub event_id: Uuid,
    pub topic_id: Uuid,
    pub topic_path: String,
    pub schema_version: i32,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EventDetail {
    pub summary: EventSummary,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub topic_id: Option<Uuid>,
    pub topic_path: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}
