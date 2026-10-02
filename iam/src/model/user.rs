use std::{collections::HashMap, str::FromStr};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    dto::user::CreateUserRequest, error::ApiError, service::password::create_password_hash,
};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMembership {
    pub role_id: Uuid,
    pub tenant_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMemberships {
    inner: Vec<UserMembership>,
}
impl Into<Vec<UserMembership>> for UserMemberships {
    fn into(self) -> Vec<UserMembership> {
        self.inner
    }
}

impl TryFrom<HashMap<String, String>> for UserMemberships {
    type Error = ApiError;
    fn try_from(value: HashMap<String, String>) -> Result<Self, Self::Error> {
        let mut res = UserMemberships { inner: Vec::new() };
        for entry in value {
            let tenant_id = Uuid::from_str(&entry.0)?;
            let role_id = Uuid::from_str(&entry.1)?;
            res.inner.push(UserMembership { role_id, tenant_id });
        }
        Ok(res)
    }
}
