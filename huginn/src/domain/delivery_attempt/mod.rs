use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DeliveryAttempt {
    pub attempt_id: Uuid,
    pub delivery_id: Uuid,
    pub attempt_number: i32,
    pub success: bool,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    /// When Outbound ran the attempt.
    pub attempted_at: DateTime<Utc>,
    /// When Muninn stored it. Shows ingestion lag.
    pub recorded_at: DateTime<Utc>,
}
