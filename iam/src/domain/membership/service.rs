use std::{str::FromStr, sync::Arc};

use crate::{
    domain::membership::{dto, repository::MembershipRepository},
    shared::error::ApiResult,
};

pub async fn upsert_membership(
    repo: &Arc<dyn MembershipRepository>,
    membership: dto::UpsertUserMembershipRequest,
) -> ApiResult<dto::MembershipDetails> {
    let res = repo.upsert(membership.into()).await?;
    Ok(res.into())
}

pub async fn list_membership(
    repo: &Arc<dyn MembershipRepository>,
    user_id: &str,
) -> ApiResult<dto::UserMemberships> {
    let user_id = uuid::Uuid::from_str(user_id)?;
    let res = repo.list(user_id).await?;
    Ok(res.into())
}
