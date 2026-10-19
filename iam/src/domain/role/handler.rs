use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

use crate::{
    domain::role::{
        dto::{Role, Roles},
        service,
    },
    shared::{
        auth::context::AuthContext,
        error::{ApiError, ApiResult},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_roles))
        .route("/{slug}", get(get_role_by_slug))
}

async fn get_roles(
    auth_context: AuthContext,
    State(app_state): State<AppState>,
) -> ApiResult<Json<Roles>> {
    auth_context.assert_is_admin()?;
    let res = service::list_roles(&app_state.role_repo).await?;
    Ok(Json(res))
}
async fn get_role_by_slug(
    auth_context: AuthContext,
    State(app_state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<Role>> {
    auth_context.assert_is_admin()?;
    let res = service::get_role_by_slug(&app_state.role_repo, &slug)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(res))
}
