use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Current schema version for tenant projection events.
pub const TENANT_PROJECTION_SCHEMA_VERSION: i16 = 1;

/// Event type stored in the PostgreSQL outbox.
pub const TENANT_PROJECTION_EVENT: &str = "tenant.projection.refresh";

/// A request to rebuild the Redis projection for a tenant.
///
/// This event deliberately carries no projection data. The worker reads the
/// latest authoritative state from PostgreSQL when processing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantProjectionEvent {
    pub tenant_id: Uuid,
}

impl TenantProjectionEvent {
    pub fn new(tenant_id: Uuid) -> Self {
        Self { tenant_id }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedEvent {
    pub schema_version: u16,
    pub event_id: Uuid,
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
    pub topic_path: String,
    // This field is for the case in which use user wants to send the event to only specified destinations
    // by default it will be sent to  all the linked destination.
    pub destinations: Vec<Uuid>,
    pub payload: Value,
}

impl PublishedEvent {
    pub fn new(
        tenant_id: Uuid,
        topic_id: Uuid,
        destinations: Vec<Uuid>,
        topic_path: String,
        payload: Value,
    ) -> Self {
        Self {
            schema_version: 1,
            event_id: Uuid::new_v4(),
            tenant_id,
            topic_id,
            destinations,
            topic_path,
            payload,
        }
    }
}
