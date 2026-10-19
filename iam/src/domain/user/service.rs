use std::sync::Arc;

use crate::{
    domain::user::{
        dto::{CreateUserRequest, UpdateUserRequest, UserFilter, UserResponse},
        model::NewUser,
        repository::UserRepository,
    },
    shared::{
        error::{ApiError, ApiResult},
        pagination::{PaginatedList, Pagination},
    },
};

pub async fn create_user(
    repo: &Arc<dyn UserRepository>,
    new_user: CreateUserRequest,
) -> ApiResult<UserResponse> {
    let new_user: NewUser = new_user.into();
    let user = repo.create(&new_user).await?;
    Ok(user.into())
}

pub async fn list_users(
    repo: &Arc<dyn UserRepository>,
    filters: &UserFilter,
    pagination: &Pagination,
) -> ApiResult<PaginatedList<UserResponse>> {
    let result = repo.list(filters, pagination).await?;
    Ok(PaginatedList {
        list: result.list.into_iter().map(Into::into).collect(),
        page_info: result.page_info,
    })
}

pub async fn get_user(repo: &Arc<dyn UserRepository>, id: uuid::Uuid) -> ApiResult<UserResponse> {
    let res = repo.get(id).await?.ok_or(ApiError::NotFound)?;
    Ok(res.into())
}

pub async fn update_user(
    repo: &Arc<dyn UserRepository>,
    id: uuid::Uuid,
    req: UpdateUserRequest,
) -> ApiResult<UserResponse> {
    let mut user = repo.get(id).await?.ok_or(ApiError::NotFound)?;
    if let Some(full_name) = req.full_name {
        user.full_name = full_name;
    }
    let updated = repo.update(&user).await?;
    Ok(updated.into())
}
