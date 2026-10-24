use chrono::Utc;
use serde::Serialize;
use sqlx::prelude::FromRow;

pub struct NewTopic {
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub tenant_id: uuid::Uuid,
    pub created_by: String,
}

#[derive(Serialize, FromRow)]
pub struct Topic {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub is_active: bool,
    pub tenant_id: uuid::Uuid,
    pub created_by: String,
    pub last_updated_by: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

pub struct DestinationLink {
    pub destination_id: uuid::Uuid,
    pub topic_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub created_by: String,
}
