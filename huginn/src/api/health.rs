use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde_json::{Value, json};
use tracing::error;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(|| async { "ok" })) // liveness: process is up
        .route("/readyz", get(ready)) // readiness: storage reachable
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.ready.check().await {
        Ok(()) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(e) => {
            // Log the detail, but don't expose connection info to callers.
            error!(error = %e, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "not_ready" })),
            )
        }
    }
}
