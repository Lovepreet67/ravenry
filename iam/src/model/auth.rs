// model/claims.rs
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn new(user: &crate::model::user::User, ttl_hours: i64) -> Self {
        let now = Utc::now();
        Self {
            sub: user.id,
            username: user.username.clone(),
            iat: now.timestamp() as usize,
            exp: (now + chrono::Duration::hours(ttl_hours)).timestamp() as usize,
        }
    }
}
