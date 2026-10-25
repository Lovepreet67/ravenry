use anyhow::{Result, bail};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use uuid::Uuid;

use iam_contracts::events::{TENANT_PROJECTION_EVENT, TENANT_PROJECTION_SCHEMA_VERSION};

use crate::platform::{
    outbox::event::OutboxEvent,
    projection::{rebuild, writer},
};

pub async fn process_tenant_projection(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    event: OutboxEvent,
) -> Result<()> {
    if event.event_type != TENANT_PROJECTION_EVENT {
        bail!("Unsupported outbox event type: {}", event.event_type);
    }

    if event.schema_version != TENANT_PROJECTION_SCHEMA_VERSION {
        bail!(
            "Unsupported schema version {} for event {}",
            event.schema_version,
            event.event_type
        );
    }

    let tenant_id: Uuid = event.aggregate_id;

    let projection = rebuild::rebuild(pool, tenant_id).await?;

    writer::write_projection(redis, tenant_id, projection.as_ref()).await?;

    Ok(())
}
