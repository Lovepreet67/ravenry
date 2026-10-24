use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use uuid::Uuid;

use crate::{
    domain::destination::dto::{
        CreateDestinationRequest, Destination, DestinationFilter, UpdateDestinationRequest,
    },
    domain::destination::service,
    shared::auth::context::AuthContext,
    shared::{
        error::ApiResult,
        pagination::{PaginatedList, Pagination},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenants/{tenant_id}/destinations",
            get(list_destinations).post(create_destination),
        )
        .route(
            "/tenants/{tenant_id}/destinations/{destination_id}",
            get(get_destination).patch(update_destination),
        )
}

async fn create_destination(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<CreateDestinationRequest>,
) -> ApiResult<(StatusCode, Json<Destination>)> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let created =
        service::create(&state.destination_repo, tenant_id, &auth.get_email(), req).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn list_destinations(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(tenant_id): Path<Uuid>,
    Query(filter): Query<DestinationFilter>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<PaginatedList<Destination>>> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let result = service::list(&state.destination_repo, tenant_id, &filter, &pagination).await?;
    Ok(Json(result))
}

async fn get_destination(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, destination_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Destination>> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let destination = service::get(&state.destination_repo, tenant_id, destination_id).await?;
    Ok(Json(destination))
}

async fn update_destination(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, destination_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateDestinationRequest>,
) -> ApiResult<Json<Destination>> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let updated = service::update(
        &state.destination_repo,
        tenant_id,
        &auth.get_email(),
        destination_id,
        req,
    )
    .await?;
    Ok(Json(updated))
}
