use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// An outbox event fetched from PostgreSQL and ready for processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    pub id: i64,
    pub event_type: String,
    pub schema_version: i16,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub attempts: i32,
}

/// Fields required to insert a new outbox event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOutboxEvent {
    pub event_type: String,
    pub schema_version: i16,
    pub aggregate_id: Uuid,
    pub payload: Value,
}

impl NewOutboxEvent {
    /// Create a tenant projection refresh event.
    pub fn tenant_projection_refresh(tenant_id: Uuid) -> Self {
        Self {
            event_type: iam_contracts::events::TENANT_PROJECTION_EVENT.to_owned(),
            schema_version: iam_contracts::events::TENANT_PROJECTION_SCHEMA_VERSION,
            aggregate_id: tenant_id,
            payload: serde_json::json!({
                "tenant_id": tenant_id,
            }),
        }
    }
}
