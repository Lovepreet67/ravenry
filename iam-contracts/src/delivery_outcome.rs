use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOutcome {
    pub schema_version: u16,
    pub delivery_id: Uuid,
    pub attempt_id: Uuid,
    pub attempt: u32,
    pub event_id: Uuid,
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
    pub destination_id: Uuid,
    pub success: bool,
    pub will_retry: bool,
    pub status_code: Option<u16>,
    pub error: Option<String>,
    pub attempted_at: DateTime<Utc>,
}
