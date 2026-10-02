use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    dto::{
        user::{UserFilter, UserResponse},
        utils::{PaginatedList, Pagination},
    },
    error::ApiResult,
    middleware::AdminAuth,
    service::user::list::list as list_service,
    state::AppState,
};

pub async fn list(
    _admin: AdminAuth,
    State(mut state): State<AppState>,
    Query(pagination): Query<Pagination>,
    Query(filter): Query<UserFilter>,
) -> ApiResult<Json<PaginatedList<UserResponse>>> {
    let x = list_service(&filter, &pagination, &mut state.db_client).await?;
    let res = Json::from(x);
    Ok(res)
}
