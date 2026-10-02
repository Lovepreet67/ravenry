use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    controller::user::{create, list::list, update},
    state::AppState,
};

pub async fn get_user_router() -> Router {
    let state = AppState::new().await;
    Router::new()
        .route("/list", get(list))
        .route("/create", post(create::create))
        .route("/membership", post(update::add_membership))
        .with_state(state)
}
