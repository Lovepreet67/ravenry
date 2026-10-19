use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use validator::Validate;

use crate::{
    domain::tenant::{dto::*, service},
    shared::{
        error::ApiResult,
        pagination::{PaginatedList, Pagination},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tenants", post(create_tenant).get(list_tenants))
        .route("/tenants/{id}", get(get_tenant).patch(update_tenant))
}

async fn create_tenant(
    State(state): State<AppState>,
    Json(req): Json<CreateTenantRequest>,
) -> ApiResult<Json<TenantResponse>> {
    req.validate()?;
    let tenant = service::create_tenant(&state.tenant_repo, req).await?;
    Ok(Json(tenant))
}

async fn get_tenant(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TenantResponse>> {
    let tenant = service::get_tenant(&state.tenant_repo, &id).await?;
    Ok(Json(tenant))
}

async fn list_tenants(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
    Query(filter): Query<TenantFilter>,
) -> ApiResult<Json<PaginatedList<TenantResponse>>> {
    let result = service::list_tenants(&state.tenant_repo, &pagination, &filter).await?;
    Ok(Json(result))
}

async fn update_tenant(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTenantRequest>,
) -> ApiResult<Json<TenantResponse>> {
    let tenant = service::update_tenant(&state.tenant_repo, &id, req).await?;
    Ok(Json(tenant))
}
