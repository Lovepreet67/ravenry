use serde::{Deserialize, Serialize};

use crate::{domain::membership::model::MembershipDetails, shared::auth::context::AuthContext};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub username: String,
    pub email: String,
    pub token: String,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub is_admin: bool,
    pub username: String,
    pub email: String,
    pub memberships: Vec<MembershipDetails>,
}

impl From<AuthContext> for UserInfo {
    fn from(value: AuthContext) -> Self {
        match value {
            AuthContext::Admin => Self {
                is_admin: true,
                username: "not_applicable".into(),
                email: "not_applicable".into(),
                memberships: Vec::new(),
            },
            AuthContext::User(val) => Self {
                is_admin: false,
                username: val.username,
                email: val.email,
                memberships: val.memberships,
            },
        }
    }
}
