use chrono::Utc;
use serde::Serialize;
use sqlx::prelude::FromRow;

use crate::domain::destination::common::DestinationType;

pub struct NewDestination {
    pub name: String,
    pub kind: DestinationType,
    pub description: Option<String>,
    pub destination_url: String,
    pub delivery_timeout_seconds: i32,
    pub tenant_id: uuid::Uuid,
    pub created_by: String,
}

#[derive(Serialize, FromRow)]
pub struct Destination {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kind: DestinationType,
    pub destination_url: String,
    pub is_active: bool,
    pub delivery_timeout_seconds: i32,
    pub tenant_id: uuid::Uuid,
    pub created_by: String,
    pub last_updated_by: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
