use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    Error::ApiResult,
    dto::{
        user::{UserFilter, UserResponse},
        utils::{PaginatedList, Pagination},
    },
    service::user::list::list as list_service,
    state::AppState,
};

pub async fn list(
    State(mut state): State<AppState>,
    Query(pagination): Query<Pagination>,
    Query(filter): Query<UserFilter>,
) -> ApiResult<Json<PaginatedList<UserResponse>>> {
    let x = list_service(&filter, &pagination, &mut state.db_client).await?;
    let res = Json::from(x);
    Ok(res)
}
