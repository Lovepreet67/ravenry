use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use iam_contracts::auth::context::AuthContext;
use uuid::Uuid;

use super::dto::{EventDetailDto, EventListQuery, EventSummaryDto, ListResponse};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tenants/{tenant_id}/events", get(list_events))
        .route("/tenants/{tenant_id}/events/{event_id}", get(get_event))
}

async fn list_events(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(tenant_id): Path<Uuid>,
    Query(query): Query<EventListQuery>,
) -> AppResult<Json<ListResponse<EventSummaryDto>>> {
    // Authorize first: nothing below runs for a caller without access.
    let scope = state.scope(&auth, tenant_id)?;
    let (filter, page) = query.into_parts()?;

    let result = state.events.list(&scope, &filter, &page).await?;
    Ok(Json(ListResponse::from_page(result, EventSummaryDto::from)))
}

async fn get_event(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, event_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<EventDetailDto>> {
    let scope = state.scope(&auth, tenant_id)?;

    // Another tenant's event looks exactly like a missing one.
    let detail = state
        .events
        .get(&scope, event_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(EventDetailDto::from_domain(detail, &scope)))
}
