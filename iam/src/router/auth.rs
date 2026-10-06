use axum::{Router, routing::get, routing::post};

use crate::{controller::user::auth, state::AppState};

pub async fn get_auth_router() -> Router {
    let state = AppState::new().await;
    axum::Router::new()
        .route("/login", post(auth::login))
        .route("/me", get(auth::me))
        .route("/public-key", get(auth::get_public_key))
        .with_state(state)
}
