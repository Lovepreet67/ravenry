use chrono::Utc;
use uuid::Uuid;

use crate::{dto::user::CreateUserRequest, service::password::create_password_hash};

pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

pub struct NewUser {
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
}

impl From<CreateUserRequest> for NewUser {
    fn from(value: CreateUserRequest) -> Self {
        Self {
            full_name: value.full_name,
            email: value.email,
            username: value.username,
            password_hash: create_password_hash(&value.password),
        }
    }
}
