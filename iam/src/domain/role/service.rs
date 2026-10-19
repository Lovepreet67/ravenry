use std::sync::Arc;

use crate::{
    domain::role::{dto, repository::RoleRepository},
    shared::error::ApiResult,
};

pub async fn list_roles(role_repo: &Arc<dyn RoleRepository>) -> ApiResult<dto::Roles> {
    let res = role_repo.list().await?;
    Ok(res.into_iter().map(Into::into).collect())
}

pub async fn get_role_by_slug(
    role_repo: &Arc<dyn RoleRepository>,
    slug: &str,
) -> ApiResult<Option<dto::Role>> {
    let res = role_repo.get_by_slug(slug).await?;
    let res = res.map(Into::into);
    Ok(res)
}
