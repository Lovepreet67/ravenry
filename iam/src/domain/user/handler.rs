use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use validator::Validate;

use crate::{
    domain::user::{dto::*, service},
    shared::{
        auth::admin::AdminAuth,
        error::ApiResult,
        pagination::{PaginatedList, Pagination},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", get(get_user).patch(update_user))
}

async fn create_user(
    _admin: AdminAuth,
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> ApiResult<Json<UserResponse>> {
    req.validate()?;
    let tenant = service::create_user(&state.user_repo, req).await?;
    Ok(Json(tenant))
}

async fn get_user(
    _admin: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<UserResponse>> {
    let tenant = service::get_user(&state.user_repo, &id).await?;
    Ok(Json(tenant))
}

async fn list_users(
    _admin: AdminAuth,
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
    Query(filters): Query<UserFilter>,
) -> ApiResult<Json<PaginatedList<UserResponse>>> {
    let result = service::list_users(&state.user_repo, &filters, &pagination).await?;
    Ok(Json(result))
}

async fn update_user(
    _admin: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> ApiResult<Json<UserResponse>> {
    let tenant = service::update_user(&state.user_repo, &id, req).await?;
    Ok(Json(tenant))
}
