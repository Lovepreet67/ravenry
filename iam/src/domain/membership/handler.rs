use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};

use crate::{
    domain::membership::{dto, service},
    shared::{auth::context::AuthContext, error::ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tenants/{tenant_id}/memberships", post(upsert_membership))
        .route(
            "/users/{user_id}/memberships",
            get(list_memberships_by_user_id),
        )
}

pub async fn list_memberships_by_user_id(
    auth_context: AuthContext,
    State(app_state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> ApiResult<Json<dto::UserMemberships>> {
    auth_context.assert_user_access(user_id, None)?;
    let res = service::list_membership_by_user_id(&app_state.membership_repo, user_id).await?;
    Ok(Json(res))
}

pub async fn upsert_membership(
    auth_context: AuthContext,
    State(app_state): State<AppState>,
    Path(tenant_id): Path<uuid::Uuid>,
    Json(req): Json<dto::UpsertUserMembershipRequest>,
) -> ApiResult<Json<dto::MembershipDetails>> {
    auth_context.assert_tenant_access(tenant_id, Some("admin"))?;
    let res = service::upsert_membership(&app_state.membership_repo, tenant_id, req).await?;
    Ok(Json(res))
}
