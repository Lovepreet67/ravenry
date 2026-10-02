use axum::{Json, extract::State};

use crate::{
    dto::user::{UpsertMemebershipRequest, UpsertMemebershipResponse},
    error::ApiResult,
    middleware::AdminAuth,
    service::user::membership::create_membership as create_membership_service,
    state::AppState,
};

pub async fn add_membership(
    _admin: AdminAuth,
    State(mut state): State<AppState>,
    Json(req): Json<UpsertMemebershipRequest>,
) -> ApiResult<Json<UpsertMemebershipResponse>> {
    let x = create_membership_service(&req, &mut state.db_client, &mut state.redis_client).await?;
    Ok(Json(x))
}
