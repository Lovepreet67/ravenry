use axum::{Json, extract::State};

use crate::{
    dto::user::{CreateUserRequest, UserResponse},
    error::ApiResult,
    middleware::AdminAuth,
    service::user::create::create_user,
    state::AppState,
};

pub async fn create(
    _admin: AdminAuth,
    State(mut state): State<AppState>,
    Json(user_req): Json<CreateUserRequest>,
) -> ApiResult<Json<UserResponse>> {
    let user = create_user(user_req, &mut state.db_client, &mut state.redis_client).await?;

    let response: UserResponse = user.into();
    Ok(Json::from(response))
}
