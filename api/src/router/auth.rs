use axum::{Router, routing::post};

use crate::{controller::user::auth, state::AppState};

pub async fn get_auth_router() -> Router {
    let state = AppState::new().await;
    axum::Router::new()
        .route("/", post(auth::login))
        .with_state(state)
}
