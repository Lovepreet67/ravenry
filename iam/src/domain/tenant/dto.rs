use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Serialize, Validate)]
pub struct CreateTenantRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub slug: String,
}

#[derive(Serialize, FromRow)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTenantRequest {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TenantFilter {
    pub is_active: Option<bool>,
}
