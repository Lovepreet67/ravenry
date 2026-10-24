use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::domain::destination::common::DestinationType;

#[derive(Deserialize, Debug)]
pub struct CreateDestinationRequest {
    pub name: String,
    pub description: Option<String>,
    pub kind: DestinationType,
    pub destination_url: String,
    pub delivery_timeout_seconds: i32,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Destination {
    pub id: uuid::Uuid,
    pub name: String,
    pub kind: DestinationType,
    pub destination_url: String,
    pub is_active: bool,
    pub delivery_timeout_seconds: i32,
    pub created_by: String,
    pub last_updated_by: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateDestinationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub kind: Option<DestinationType>,
    pub destination_url: Option<String>,
    pub is_active: Option<bool>,
    pub delivery_timeout_seconds: Option<i32>,
}

#[derive(Deserialize)]
pub struct DestinationFilter {
    pub name: Option<String>,
    pub kind: Option<DestinationType>,
    pub topic_id: Option<uuid::Uuid>,
    pub linked: Option<bool>,
}
