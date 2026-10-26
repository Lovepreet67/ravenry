use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

pub type User = iam_contracts::auth::model::User;

#[derive(Debug, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewUser {
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMembership {
    pub role_id: Uuid,
    pub tenant_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMemberships {
    inner: Vec<UserMembership>,
}
impl UserMemberships {
    pub fn new(memberships: Vec<UserMembership>) -> Self {
        Self { inner: memberships }
    }
    pub fn get_memberships(&self) -> &Vec<UserMembership> {
        &self.inner
    }
}
