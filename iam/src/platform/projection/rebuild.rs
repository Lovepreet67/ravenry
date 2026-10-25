use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Complete Redis projection for a single tenant.
#[derive(Debug, Clone, Serialize)]
pub struct TenantProjection {
    pub tenant: TenantProjectionMeta,
    pub topics: Vec<TopicProjection>,
    pub destinations: Vec<DestinationProjection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TenantProjectionMeta {
    pub id: Uuid,
    pub slug: String,
    pub is_active: bool,
}

/// A topic and the destinations subscribed to it.
#[derive(Debug, Clone, Serialize)]
pub struct TopicProjection {
    pub id: Uuid,
    pub path: String,
    pub is_active: bool,
    pub destination_ids: Vec<Uuid>,
}

/// Destination configuration required by the delivery service.
#[derive(Debug, Clone, Serialize)]
pub struct DestinationProjection {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub destination_url: String,
    pub is_active: bool,
    pub delivery_timeout_seconds: i32,
}

#[derive(Debug, FromRow)]
struct TenantRow {
    id: Uuid,
    slug: String,
    is_active: bool,
}

#[derive(Debug, FromRow)]
struct TopicRow {
    id: Uuid,
    path: String,
    is_active: bool,
}

#[derive(Debug, FromRow)]
struct DestinationRow {
    id: Uuid,
    name: String,
    kind: String,
    destination_url: String,
    is_active: bool,
    delivery_timeout_seconds: i32,
}

#[derive(Debug, FromRow)]
struct SubscriptionRow {
    topic_id: Uuid,
    destination_id: Uuid,
}

/// Load the authoritative tenant state and its complete projection data.
///
/// Returns `Ok(None)` if the tenant no longer exists.
pub async fn rebuild(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Option<TenantProjection>, sqlx::Error> {
    let tenant = sqlx::query_as::<_, TenantRow>(
        r#"
        SELECT id, slug, is_active
        FROM tenants
        WHERE id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;

    let Some(tenant) = tenant else {
        return Ok(None);
    };

    let topic_rows = sqlx::query_as::<_, TopicRow>(
        r#"
        SELECT id, path, is_active
        FROM topics
        WHERE tenant_id = $1
        ORDER BY id
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    let destination_rows = sqlx::query_as::<_, DestinationRow>(
        r#"
        SELECT
            id,
            name,
            kind::TEXT AS kind,
            destination_url,
            is_active,
            delivery_timeout_seconds
        FROM destinations
        WHERE tenant_id = $1
        ORDER BY id
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    let subscription_rows = sqlx::query_as::<_, SubscriptionRow>(
        r#"
        SELECT
            j.topic_id,
            j.destination_id
        FROM topics_junc_destinations AS j
        INNER JOIN topics AS t
            ON t.id = j.topic_id
        INNER JOIN destinations AS d
            ON d.id = j.destination_id
        WHERE t.tenant_id = $1
          AND d.tenant_id = $1
        ORDER BY j.topic_id, j.destination_id
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    let mut subscriptions_by_topic: std::collections::HashMap<Uuid, Vec<Uuid>> =
        std::collections::HashMap::new();

    for subscription in subscription_rows {
        subscriptions_by_topic
            .entry(subscription.topic_id)
            .or_default()
            .push(subscription.destination_id);
    }

    let topics = topic_rows
        .into_iter()
        .map(|topic| TopicProjection {
            id: topic.id,
            path: topic.path,
            is_active: topic.is_active,
            destination_ids: subscriptions_by_topic.remove(&topic.id).unwrap_or_default(),
        })
        .collect();

    let destinations = destination_rows
        .into_iter()
        .map(|destination| DestinationProjection {
            id: destination.id,
            name: destination.name,
            kind: destination.kind,
            destination_url: destination.destination_url,
            is_active: destination.is_active,
            delivery_timeout_seconds: destination.delivery_timeout_seconds,
        })
        .collect();

    Ok(Some(TenantProjection {
        tenant: TenantProjectionMeta {
            id: tenant.id,
            slug: tenant.slug,
            is_active: tenant.is_active,
        },
        topics,
        destinations,
    }))
}
