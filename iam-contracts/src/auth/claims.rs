use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::auth::{
    config::AuthConfig,
    model::{MembershipDetails, User},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub memberships: Vec<MembershipDetails>,
    pub user_id: uuid::Uuid,
    pub email: String,
}

impl Claims {
    pub fn new(user: &User, memberships: Vec<MembershipDetails>, config: &AuthConfig) -> Self {
        let now = Utc::now().timestamp() as u64;
        let ttl = u64::from(config.jwt_ttl_hours) * 3600;
        Self {
            sub: user.username.clone(),
            email: user.email.clone(),
            user_id: user.id,
            exp: now + ttl,
            iat: now,
            memberships: memberships,
        }
    }
}
