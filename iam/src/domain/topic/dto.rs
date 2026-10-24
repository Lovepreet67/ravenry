use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct CreateTopicRequest {
    pub name: String,
    pub description: Option<String>,
    pub path: String,
}

#[derive(Deserialize, Debug, Serialize)]
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

#[derive(Deserialize, Debug)]
pub struct UpdateTopicRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct TopicFilter {
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct DestinationList {
    destination_ids: Vec<uuid::Uuid>,
}
