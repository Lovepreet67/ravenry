use axum::{
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::{
    state::AppState,
    transport::http::handlers::{health::health, publish::publish},
};

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/tenants/{tenant_id}/events", post(publish))
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
}
