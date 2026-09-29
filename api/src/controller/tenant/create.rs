use axum::{Json, extract::State};

use crate::{
    Error::ApiResult,
    dto::tenant::{CreateTenantRequest, TenantResponse},
    service::tenant::create::create_tenant,
    state::AppState,
};

pub async fn create(
    State(mut state): State<AppState>,
    Json(tenant_req): Json<CreateTenantRequest>,
) -> ApiResult<Json<TenantResponse>> {
    let tenant = create_tenant(&tenant_req, &mut state.db_client, &mut state.redis_client).await?;

    let response: TenantResponse = tenant.into();
    Ok(Json::from(response))
}
