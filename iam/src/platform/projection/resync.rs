use redis::{AsyncCommands, aio::ConnectionManager};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use iam_contracts::keys::PROJECTION_VERSION_KEY;

use crate::platform::projection::{rebuild, writer};

const TENANT_PROJECTION_KEYS_PATTERN: &str = "v1:tenant:*:projection-keys";

type ResyncResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Rebuild every tenant's Redis projection from PostgreSQL.
///
/// The readiness marker is deleted first and restored only after the entire
/// resync succeeds. If resync fails, the marker remains absent.
pub async fn resync(pool: &PgPool, connection: &mut ConnectionManager) -> ResyncResult<()> {
    // Prevent consumers from treating a partially rebuilt projection as ready.
    let _: usize = connection.del(PROJECTION_VERSION_KEY).await?;

    // Remove tracked keys from previous projections, including projections
    // belonging to tenants that no longer exist in PostgreSQL.
    clear_existing_projections(connection).await?;

    // Load the authoritative list of tenants.
    let tenant_ids = sqlx::query_scalar::<Postgres, Uuid>(
        r#"
        SELECT id
        FROM tenants
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await?;

    // Rebuild and publish each tenant's current projection.
    for tenant_id in tenant_ids {
        let projection = rebuild::rebuild(pool, tenant_id).await?;

        writer::write_projection(connection, tenant_id, projection.as_ref()).await?;
    }

    // Publish readiness only after all tenant projections have succeeded.
    let _: () = connection.set(PROJECTION_VERSION_KEY, "1").await?;

    Ok(())
}

/// Remove every previously tracked tenant projection.
///
/// SCAN is used instead of KEYS so this remains safe on a larger Redis
/// instance. Each tenant's index contains the keys created by writer.rs.
async fn clear_existing_projections(connection: &mut ConnectionManager) -> ResyncResult<()> {
    let mut cursor: u64 = 0;

    loop {
        let (next_cursor, index_keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(TENANT_PROJECTION_KEYS_PATTERN)
            .arg("COUNT")
            .arg(100)
            .query_async(connection)
            .await?;

        for index_key in index_keys {
            let tracked_keys: Vec<String> = connection.smembers(&index_key).await?;

            if !tracked_keys.is_empty() {
                let _: usize = connection.del(tracked_keys).await?;
            }

            let _: usize = connection.del(index_key).await?;
        }

        cursor = next_cursor;

        if cursor == 0 {
            break;
        }
    }

    Ok(())
}
