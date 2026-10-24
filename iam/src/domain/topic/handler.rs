use std::println;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, put},
};
use uuid::Uuid;

use crate::{
    domain::topic::dto::{CreateTopicRequest, Topic, TopicFilter, UpdateTopicRequest},
    domain::topic::service,
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
            "/tenants/{tenant_id}/topics",
            get(list_topics).post(create_topic),
        )
        .route(
            "/tenants/{tenant_id}/topics/{topic_id}",
            get(get_topic).patch(update_topic),
        )
        .route(
            "/tenants/{tenant_id}/topics/{topic_id}/destinations/{destnation_id}",
            put(put_destination).delete(delete_destination),
        )
}

async fn create_topic(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<CreateTopicRequest>,
) -> ApiResult<(StatusCode, Json<Topic>)> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let created = service::create(&state.topic_repo, tenant_id, &auth.get_email(), req).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn list_topics(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(tenant_id): Path<Uuid>,
    Query(filter): Query<TopicFilter>,
    Query(pagination): Query<Pagination>,
) -> ApiResult<Json<PaginatedList<Topic>>> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let result = service::list(&state.topic_repo, tenant_id, &filter, &pagination).await?;
    Ok(Json(result))
}

async fn get_topic(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, topic_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Topic>> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let topic = service::get(&state.topic_repo, tenant_id, topic_id).await?;
    Ok(Json(topic))
}

async fn update_topic(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, topic_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateTopicRequest>,
) -> ApiResult<Json<Topic>> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let updated = service::update(
        &state.topic_repo,
        tenant_id,
        &auth.get_email(),
        topic_id,
        req,
    )
    .await?;
    Ok(Json(updated))
}

async fn put_destination(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, topic_id, destination_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<Topic>> {
    println!("Putting");
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let topic = service::put_destination(
        &state.topic_repo,
        topic_id,
        tenant_id,
        destination_id,
        &auth.get_email(),
    )
    .await?;
    Ok(Json(topic))
}
async fn delete_destination(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((tenant_id, topic_id, destination_id)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<Topic>> {
    auth.assert_tenant_access(tenant_id, Some("admin"))?;
    let topic = service::delete_destination(
        &state.topic_repo,
        topic_id,
        tenant_id,
        destination_id,
        &auth.get_email(),
    )
    .await?;
    Ok(Json(topic))
}
