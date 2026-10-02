use axum::{Json, extract::State};

use crate::{
    dto::tenant::{CreateTenantRequest, TenantResponse},
    error::ApiResult,
    middleware::AdminAuth,
    service::tenant::create::create_tenant,
    state::AppState,
};

pub async fn create(
    _admin: AdminAuth,
    State(mut state): State<AppState>,
    Json(tenant_req): Json<CreateTenantRequest>,
) -> ApiResult<Json<TenantResponse>> {
    let tenant = create_tenant(&tenant_req, &mut state.db_client, &mut state.redis_client).await?;

    let response: TenantResponse = tenant.into();
    Ok(Json::from(response))
}
