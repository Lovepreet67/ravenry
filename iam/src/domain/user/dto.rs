use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Validate, Deserialize)]
pub struct CreateUserRequest {
    pub full_name: String,
    #[validate(email)]
    pub email: String,
    pub username: String,
    // #[validate(string(min_length = 1))]
    pub password: String,
}
#[derive(Serialize, Validate, FromRow)]
pub struct UserResponse {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize, Validate)]
pub struct UpdateUserRequest {
    pub id: Uuid,
    pub full_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserFilter {
    pub full_name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpsertMemebershipRequest {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct UpsertMemebershipResponse {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role_id: Uuid,
    pub created: bool,
}
impl UpsertMemebershipResponse {
    pub fn from_request(req: &UpsertMemebershipRequest, created: bool) -> Self {
        Self {
            user_id: req.user_id,
            tenant_id: req.tenant_id,
            role_id: req.role_id,
            created,
        }
    }
}
