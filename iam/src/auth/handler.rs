use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};

use crate::{
    auth::{
        dto::{LoginRequest, LoginResponse, UserInfo},
        service,
    },
    shared::{auth::context::AuthContext, error::ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/public-key", get(public_key))
}

async fn login(
    State(app_state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let res = service::login(
        &app_state.user_repo,
        &app_state.membership_repo,
        &app_state.config,
        app_state.jwt_signer,
        req,
    )
    .await?;
    Ok(Json(res))
}

async fn me(auth_context: AuthContext) -> ApiResult<Json<UserInfo>> {
    Ok(Json(auth_context.into()))
}
async fn public_key() -> ApiResult<&'static str> {
    let key = include_str!("../../keys/public.pem");
    Ok(key)
}
