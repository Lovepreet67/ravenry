use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::model::tenant::Tenant;

#[derive(Deserialize, Serialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
}

impl From<Tenant> for TenantResponse {
    fn from(value: Tenant) -> Self {
        Self {
            id: value.id,
            name: value.name,
            slug: value.slug,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Serialize, FromRow)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: Option<String>,
    pub slug: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
