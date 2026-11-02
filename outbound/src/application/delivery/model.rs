use iam_contracts::delivery_outcome::DeliveryOutcome;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Pending,
    Processing,
    RetryScheduled,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireResult {
    Acquired { attempt: u32 },
    Busy,
    Missing,
    AlreadySucceeded,
    AlreadyFailed,
    InvalidState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteAttemptResult {
    Completed,
    Missing,
    NotProcessing,
    StaleAttempt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRequest {
    pub delivery_id: Uuid,
    pub event_id: Uuid,
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
    pub destination_id: Uuid,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqMessage {
    pub outcome: DeliveryOutcome,
    pub request: DeliveryRequest,
}
