use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use validator::Validate;

use crate::{
    domain::tenant::{dto::*, service},
    shared::{
        auth::context::AuthContext,
        error::ApiResult,
        pagination::{PaginatedList, Pagination},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tenants", post(create_tenant).get(list_tenants))
        .route(
            "/tenants/{id}",
            get(get_tenant).patch(update_tenant).delete(delete_tenant),
        )
}

async fn create_tenant(
    auth_context: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<CreateTenantRequest>,
) -> ApiResult<Json<TenantResponse>> {
    auth_context.assert_is_admin()?;
    req.validate()?;
    let tenant = service::create_tenant(&state.tenant_repo, &state.role_repo, req).await?;
    Ok(Json(tenant))
}

async fn list_tenants(
    auth_context: AuthContext,
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
    Query(filter): Query<TenantFilter>,
) -> ApiResult<Json<PaginatedList<TenantResponse>>> {
    auth_context.assert_is_admin()?;
    let result = service::list_tenants(&state.tenant_repo, &pagination, &filter).await?;
    Ok(Json(result))
}

async fn delete_tenant(auth_context: AuthContext) -> ApiResult<&'static str> {
    auth_context.assert_is_admin()?;
    return Ok("Not Implemented");
}

async fn get_tenant(
    auth_context: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<TenantResponse>> {
    auth_context.assert_tenant_access(id, None)?;
    let tenant = service::get_tenant(&state.tenant_repo, id).await?;
    Ok(Json(tenant))
}

async fn update_tenant(
    auth_context: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<UpdateTenantRequest>,
) -> ApiResult<Json<TenantResponse>> {
    auth_context.assert_tenant_access(id, Some("owner"))?;
    let tenant = service::update_tenant(&state.tenant_repo, id, req).await?;
    Ok(Json(tenant))
}
