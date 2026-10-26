use axum::{
    extract::{Path, State},
    Json,
};
use iam_contracts::auth::context::AuthContext;
use serde_json::{json, Value};

use crate::{
    application::publish::{command::PublishCommand, service::PublishService},
    state::AppState,
    transport::http::error::HttpError,
};

pub async fn publish(
    State(state): State<AppState>,
    auth_context: AuthContext,
    Path(tenant_id): Path<uuid::Uuid>,
    Json(command): Json<PublishCommand>,
) -> Result<Json<Value>, HttpError> {
    // Tenant context must come from a trusted, validated identity.
    // Replace this with the actual tenant context from your IAM contract.
    auth_context.assert_tenant_any_access(tenant_id, &state.config.iam.publish_roles)?;
    let service = PublishService::new(
        state.queue.clone(),
        state.projections.clone(),
        state.publish_roles.clone(),
    );

    let message_id = service
        .publish(tenant_id, command)
        .await
        .map_err(HttpError::Publish)?;

    Ok(Json(json!({
        "status": "published",
        "message_id": message_id
    })))
}
