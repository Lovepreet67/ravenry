use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use validator::Validate;

use crate::{
    domain::user::{dto::*, service},
    shared::{
        auth::context::AuthContext,
        error::ApiResult,
        pagination::{PaginatedList, Pagination},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", post(create_user).get(list_users))
        .route(
            "/users/{id}",
            get(get_user).patch(update_user).delete(delete_user),
        )
}

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> ApiResult<Json<UserResponse>> {
    req.validate()?;
    let tenant = service::create_user(&state.user_repo, req).await?;
    Ok(Json(tenant))
}

async fn get_user(
    auth_context: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<UserResponse>> {
    auth_context.assert_user_access(id, None)?;
    let tenant = service::get_user(&state.user_repo, id).await?;
    Ok(Json(tenant))
}

async fn list_users(
    auth_context: AuthContext,
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
    Query(filters): Query<UserFilter>,
) -> ApiResult<Json<PaginatedList<UserResponse>>> {
    auth_context.assert_is_admin()?;
    let result = service::list_users(&state.user_repo, &filters, &pagination).await?;
    Ok(Json(result))
}

async fn update_user(
    auth_context: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> ApiResult<Json<UserResponse>> {
    auth_context.assert_user_access(id, None)?;
    let tenant = service::update_user(&state.user_repo, id, req).await?;
    Ok(Json(tenant))
}

async fn delete_user(
    auth_context: AuthContext,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<&'static str> {
    auth_context.assert_user_access(id, None)?;
    Ok("Not Implemented")
}
