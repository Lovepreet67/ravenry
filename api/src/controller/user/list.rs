use axum::{Json, extract::State};

use crate::{
    Error::ApiResult, dto::user::UserResponse, service::user::list::list as list_service,
    state::AppState,
};

pub async fn list(State(mut state): State<AppState>) -> ApiResult<Json<Vec<UserResponse>>> {
    let x = list_service(&mut state.db_client).await?;
    let res = Json::from(x);
    Ok(res)
}
pub async fn list_id() -> &'static str {
    "testing"
}
