use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use iam_contracts::auth::context::AuthContext;
use serde::Serialize;
use uuid::Uuid;

use super::dto::{AttemptDto, DeliveryDto, DeliveryListQuery, ListResponse};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tenants/{tenant_id}/deliveries", get(list_deliveries))
        .route(
            "/tenants/{tenant_id}/events/{event_id}/deliveries",
            get(list_event_deliveries),
        )
        .route(
            "/tenants/{tenant_id}/deliveries/{delivery_id}/attempts",
            get(delivery_attempts),
        )
}

/// Tenant-wide delivery log, filterable by event, destination, status, and time.
async fn list_deliveries(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(tenant_id): Path<Uuid>,
    Query(query): Query<DeliveryListQuery>,
) -> AppResult<Json<ListResponse<DeliveryDto>>> {
    let scope = state.scope(&auth, tenant_id)?;
    let (filter, page) = query.into_parts()?;

    let result = state.deliveries.list(&scope, &filter, &page).await?;
    Ok(Json(ListResponse::from_page(result, DeliveryDto::from)))
}

/// One row per destination for a single event. The path's event id always
/// wins over any `event_id` in the query string.
async fn list_event_deliveries(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, event_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<DeliveryListQuery>,
) -> AppResult<Json<ListResponse<DeliveryDto>>> {
    let scope = state.scope(&auth, tenant_id)?;

    // 404 for a missing event or one owned by another tenant. Without this
    // check, "not yours" and "no deliveries yet" would look the same.
    state
        .events
        .get(&scope, event_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let (mut filter, page) = query.into_parts()?;
    filter.event_id = Some(event_id);

    let result = state.deliveries.list(&scope, &filter, &page).await?;
    Ok(Json(ListResponse::from_page(result, DeliveryDto::from)))
}

#[derive(Debug, Serialize)]
struct AttemptHistoryDto {
    delivery: DeliveryDto,
    /// Newest attempt first. Bounded by the retry policy, so not paginated.
    attempts: Vec<AttemptDto>,
}

async fn delivery_attempts(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, delivery_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<AttemptHistoryDto>> {
    let scope = state.scope(&auth, tenant_id)?;

    // Get the delivery first: the attempts query returns an empty list for
    // unknown and foreign deliveries alike.
    let delivery = state
        .deliveries
        .get(&scope, delivery_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let attempts = state
        .attempts
        .list_for_delivery(&scope, delivery_id)
        .await?;

    Ok(Json(AttemptHistoryDto {
        delivery: delivery.into(),
        attempts: attempts.into_iter().map(AttemptDto::from).collect(),
    }))
}
