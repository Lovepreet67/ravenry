// model/claims.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::user::{UserMembership, UserMemberships};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
    pub memberships: Vec<UserMembership>,
}
