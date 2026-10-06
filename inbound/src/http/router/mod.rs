use axum::{routing::post, Router};
use tower_http::trace::TraceLayer;

use crate::{http::controller::ingest_event, AppState};

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/events/{id}", post(ingest_event))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
