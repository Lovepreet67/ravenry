use std::sync::Arc;

use iam_contracts::destination_delivery::DeliveryOutcome;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::ResolvedDestination;

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
    pub destination_config: Arc<ResolvedDestination>,
    pub curr_attempt: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqMessage {
    pub outcome: DeliveryOutcome,
    pub request: DeliveryRequest,
}
