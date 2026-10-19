use crate::domain::membership::{dto, model};
impl model::UserMembership {
    pub fn from_req(value: dto::UpsertUserMembershipRequest, tenant_id: uuid::Uuid) -> Self {
        Self {
            user_id: value.user_id,
            tenant_id,
            role_id: value.role_id,
        }
    }
}

impl From<model::MembershipDetails> for dto::MembershipDetails {
    fn from(value: model::MembershipDetails) -> Self {
        Self {
            tenant_id: value.tenant_id,
            tenant_name: value.tenant_name,
            tenant_slug: value.tenant_slug,
            role_slug: value.role_slug,
            role_id: value.role_id,
            role_title: value.role_title,
        }
    }
}

impl From<model::UserMembershipDetails> for dto::UserMemberships {
    fn from(value: model::UserMembershipDetails) -> Self {
        Self {
            user_id: value.user_id,
            memberships: value.memberships.into_iter().map(Into::into).collect(),
        }
    }
}
