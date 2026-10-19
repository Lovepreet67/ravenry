use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow)]
pub struct UserMembership {
    pub user_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub role_id: uuid::Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MembershipDetails {
    pub tenant_id: uuid::Uuid,
    pub tenant_name: String,
    pub role_id: uuid::Uuid,
    pub role_title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMembershipDetails {
    pub user_id: uuid::Uuid,
    pub memberships: Vec<MembershipDetails>,
}
