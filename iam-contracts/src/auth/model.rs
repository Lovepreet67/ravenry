use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MembershipDetails {
    pub tenant_id: uuid::Uuid,
    pub tenant_name: String,
    pub tenant_slug: String,
    pub role_id: uuid::Uuid,
    pub role_title: String,
    pub role_slug: String,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: uuid::Uuid,
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
