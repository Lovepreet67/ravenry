use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Role {
    pub id: uuid::Uuid,
    pub title: String,
    pub description: Option<String>,
    pub slug: String,
    pub created_at: chrono::DateTime<Utc>,
}

pub type Roles = Vec<Role>;
