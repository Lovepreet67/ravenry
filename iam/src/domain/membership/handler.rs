use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

use crate::{
    domain::membership::{dto, service},
    shared::error::ApiResult,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/memberships/{user_id}",
        get(list_memberships).post(upsert_membership),
    )
}

pub async fn list_memberships(
    State(app_state): State<AppState>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<dto::UserMemberships>> {
    let res = service::list_membership(&app_state.membership_repo, &user_id).await?;
    Ok(Json(res))
}

pub async fn upsert_membership(
    State(app_state): State<AppState>,
    Json(req): Json<dto::UpsertUserMembershipRequest>,
) -> ApiResult<Json<dto::MembershipDetails>> {
    let res = service::upsert_membership(&app_state.membership_repo, req).await?;
    Ok(Json(res))
}
