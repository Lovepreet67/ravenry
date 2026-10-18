use std::{collections::HashMap, str::FromStr};

use uuid::Uuid;

use crate::{
    domain::user::{
        dto::{CreateUserRequest, UserResponse},
        model::{NewUser, User, UserMembership, UserMemberships},
    },
    shared::{auth::crypto::create_password_hash, error::ApiError},
};

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

impl Into<Vec<UserMembership>> for UserMemberships {
    fn into(self) -> Vec<UserMembership> {
        self.get_memberships().clone()
    }
}

impl TryFrom<HashMap<String, String>> for UserMemberships {
    type Error = ApiError;
    fn try_from(value: HashMap<String, String>) -> Result<Self, Self::Error> {
        let mut memberships = Vec::new();
        for entry in value {
            let tenant_id = Uuid::from_str(&entry.0)?;
            let role_id = Uuid::from_str(&entry.1)?;
            memberships.push(UserMembership { role_id, tenant_id });
        }
        Ok(UserMemberships::new(memberships))
    }
}
