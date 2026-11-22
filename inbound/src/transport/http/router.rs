use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::{
    state::AppState,
    transport::http::handlers::{health::health, publish::publish, retry::retry},
};

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/tenants/{tenant_id}/events", post(publish))
        .route("/tenants/{tenant_id}/retry", post(retry))
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(mimir::trace::server_span))
}
