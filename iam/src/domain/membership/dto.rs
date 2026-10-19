use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct UpsertUserMembershipRequest {
    pub user_id: uuid::Uuid,
    pub role_id: uuid::Uuid,
}

#[derive(Debug, Serialize)]
pub struct MembershipDetails {
    pub tenant_id: uuid::Uuid,
    pub tenant_name: String,
    pub tenant_slug: String,
    pub role_id: uuid::Uuid,
    pub role_title: String,
    pub role_slug: String,
}

#[derive(Debug, Serialize)]
pub struct UserMemberships {
    pub user_id: uuid::Uuid,
    pub memberships: Vec<MembershipDetails>,
}
