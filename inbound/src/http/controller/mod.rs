use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::{http::middleware::AuthenticatedTenant, AppState};

#[derive(Debug, Deserialize)]
pub struct IngestEventRequest {
    pub topic: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct IngestEventResponse {
    pub event_id: String,
}

pub async fn ingest_event(
    tenant: AuthenticatedTenant,
    State(state): State<AppState>,
    Json(req): Json<IngestEventRequest>,
) -> Result<(StatusCode, Json<IngestEventResponse>), (StatusCode, String)> {
    // envelope the tenant_id into the message — the delivery worker needs it,
    // and the caller must never be able to spoof it via the request body
    let envelope = serde_json::json!({
        "tenant_id": tenant.tenant_id,
        "topic": req.topic,
        "payload": req.payload,
    });

    let bytes = serde_json::to_vec(&envelope)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let event_id = state
        .queue
        .publish("events", &bytes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::ACCEPTED, Json(IngestEventResponse { event_id })))
}
