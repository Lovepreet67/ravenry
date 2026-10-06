use std::{collections::HashMap, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
}

pub struct NewUser {
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
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
