use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    controller::user::{
        create,
        list::{list, list_id},
    },
    state::AppState,
};

pub async fn get_user_router() -> Router {
    let state = AppState::new().await;
    Router::new()
        .route("/list", get(list))
        .route("/list/{user_id}", get(list_id))
        .route("/create", post(create::create))
        .with_state(state)
}
