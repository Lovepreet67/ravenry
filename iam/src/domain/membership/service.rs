use std::{str::FromStr, sync::Arc};

use crate::{
    domain::membership::{dto, model, repository::MembershipRepository},
    shared::error::ApiResult,
};

pub async fn upsert_membership(
    repo: &Arc<dyn MembershipRepository>,
    tenant_id: uuid::Uuid,
    membership: dto::UpsertUserMembershipRequest,
) -> ApiResult<dto::MembershipDetails> {
    let new_membership = model::UserMembership::from_req(membership, tenant_id);
    let res = repo.upsert(new_membership).await?;
    Ok(res.into())
}

pub async fn list_membership_by_user_id(
    repo: &Arc<dyn MembershipRepository>,
    user_id: uuid::Uuid,
) -> ApiResult<dto::UserMemberships> {
    let res = repo.list(user_id).await?;
    Ok(res.into())
}
