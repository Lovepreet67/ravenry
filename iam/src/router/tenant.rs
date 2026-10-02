use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    controller::tenant::{create, list::list},
    state::AppState,
};

pub async fn get_tenant_router() -> Router {
    let state = AppState::new().await;
    Router::new()
        .route("/list", get(list))
        .route("/create", post(create::create))
        .with_state(state)
}
