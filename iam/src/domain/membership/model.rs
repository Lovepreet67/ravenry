use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(FromRow)]
pub struct UserMembership {
    pub user_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub role_id: uuid::Uuid,
}

pub type MembershipDetails = iam_contracts::auth::model::MembershipDetails;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMembershipDetails {
    pub user_id: uuid::Uuid,
    pub memberships: Vec<MembershipDetails>,
}
