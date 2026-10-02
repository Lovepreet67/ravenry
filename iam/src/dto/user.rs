use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::{
    dto::utils::Pagination,
    error::{ApiError::Gen, ApiResult},
    model::user::User,
};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password: String,
}
#[derive(Serialize, FromRow)]
pub struct UserResponse {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            full_name: value.full_name,
            email: value.email,
            username: value.username,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
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
