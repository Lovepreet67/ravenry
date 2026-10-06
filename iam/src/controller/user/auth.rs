use axum::{Json, extract::State};

use crate::dto::auth::LoginResponse;
use crate::middleware::CurrentUser;
use crate::service::auth::login as login_service;
use crate::{dto::auth::LoginRequest, error::ApiResult, state::AppState};

pub async fn login(
    State(mut state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let x = login_service(&req, &mut state.db_client, &mut state.redis_client).await?;
    Ok(Json::from(x))
}

pub async fn me(user: CurrentUser) -> ApiResult<Json<CurrentUser>> {
    Ok(Json(user))
}
pub async fn get_public_key() -> &'static str {
    include_str!("../../../keys/public.pem")
}
