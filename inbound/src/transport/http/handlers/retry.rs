use axum::{
    extract::{Path, State},
    Json,
};
use iam_contracts::auth::context::AuthContext;
use serde_json::{json, Value};

use crate::{
    application::retry::{command::RetryCommand, service::RetryService},
    state::AppState,
    transport::http::error::HttpError,
};

pub async fn retry(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Path(tenant_id): Path<uuid::Uuid>,
    Json(command): Json<RetryCommand>,
) -> Result<Json<Value>, HttpError> {
    // Tenant context must come from a trusted, validated identity.
    // Replace this with the actual tenant context from your IAM contract.
    auth_context.assert_tenant_any_access(tenant_id, &state.config.iam.publish_roles)?;
    let service = RetryService::new(state.queue.clone(), state.projections.clone());

    let message_id = service
        .initiate_retry(tenant_id, command)
        .await
        .map_err(HttpError::Retry)?;

    Ok(Json(json!({
        "status": "Queued",
        "message_id": message_id
    })))
}
