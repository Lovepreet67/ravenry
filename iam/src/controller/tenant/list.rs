use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    dto::utils::{PaginatedList, Pagination},
    error::ApiResult,
    middleware::AdminAuth,
    model::tenant::Tenant,
    service::tenant::list::list as list_service,
    state::AppState,
};

pub async fn list(
    _admin: AdminAuth,
    State(mut state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<PaginatedList<Tenant>>> {
    let x = list_service(&pagination, &mut state.db_client).await?;
    let res = Json::from(x);
    Ok(res)
}
