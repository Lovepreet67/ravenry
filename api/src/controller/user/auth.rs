use axum::{Json, extract::State};

use crate::dto::auth::LoginResponse;
use crate::service::auth::login as login_service;
use crate::{Error::ApiResult, dto::auth::LoginRequest, state::AppState};

pub async fn login(
    State(mut state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let x = login_service(&req, &mut state.db_client, &mut state.redis_client).await?;
    Ok(Json::from(x))
}
