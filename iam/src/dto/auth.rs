// dto/user.rs (additions)
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    pub username: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub jwt: String,
    pub expires_at: usize, // handy for the client to know when to refresh
}

#[derive(Debug, Serialize)]
pub struct AuthenticateResponse {
    pub user_id: uuid::Uuid,
    pub user_name: String,
    pub tenant_id: uuid::Uuid,
}
