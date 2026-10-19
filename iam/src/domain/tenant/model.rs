use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

pub struct NewTenant {
    pub name: String,
    pub slug: String,
    pub default_role_id: Uuid,
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub is_active: bool,
    pub default_role_id: Uuid,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
