use serde_json::Value;
use sqlx::{FromRow, Postgres, Transaction, postgres::PgPool, types::Json};
use uuid::Uuid;

use super::event::{NewOutboxEvent, OutboxEvent};

/// Duration for which a claimed event remains unavailable to other workers.
const DEFAULT_LEASE_SECONDS: i64 = 60;

/// Database representation of an outbox event.
#[derive(Debug, FromRow)]
struct OutboxEventRow {
    id: i64,
    event_type: String,
    schema_version: i16,
    aggregate_id: Uuid,
    payload: Json<Value>,
    attempts: i32,
}

impl From<OutboxEventRow> for OutboxEvent {
    fn from(row: OutboxEventRow) -> Self {
        Self {
            id: row.id,
            event_type: row.event_type,
            schema_version: row.schema_version,
            aggregate_id: row.aggregate_id,
            payload: row.payload.0,
            attempts: row.attempts,
        }
    }
}

/// Insert an event using an existing transaction.
///
/// Call this before committing the transaction that modifies the corresponding
/// business data. If the transaction rolls back, the event rolls back too.
pub async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    event: &NewOutboxEvent,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<Postgres, i64>(
        r#"
        INSERT INTO outbox_events (
            event_type,
            schema_version,
            aggregate_id,
            payload
        )
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(&event.event_type)
    .bind(event.schema_version)
    .bind(event.aggregate_id)
    .bind(Json(event.payload.clone()))
    .fetch_one(&mut **tx)
    .await
}

/// Claim a batch of eligible events.
///
/// `available_at` is advanced to create a temporary lease. If the worker
/// crashes, the event becomes eligible again after the lease expires.
///
/// Only the oldest unprocessed event for each aggregate can be claimed.
/// This prevents later events for the same tenant from overtaking it.
pub async fn claim_batch(pool: &PgPool, batch_size: i64) -> Result<Vec<OutboxEvent>, sqlx::Error> {
    if batch_size <= 0 {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<Postgres, OutboxEventRow>(
        r#"
        WITH candidates AS (
            SELECT e.id
            FROM outbox_events AS e
            WHERE e.processed_at IS NULL
              AND e.available_at <= NOW()
              AND NOT EXISTS (
                  SELECT 1
                  FROM outbox_events AS earlier
                  WHERE earlier.aggregate_id = e.aggregate_id
                    AND earlier.processed_at IS NULL
                    AND earlier.id < e.id
              )
            ORDER BY e.id
            LIMIT $1
            FOR UPDATE OF e SKIP LOCKED
        )
        UPDATE outbox_events AS e
        SET
            attempts = e.attempts + 1,
            available_at =
                NOW() + ($2::BIGINT * INTERVAL '1 second')
        FROM candidates AS c
        WHERE e.id = c.id
        RETURNING
            e.id,
            e.event_type,
            e.schema_version,
            e.aggregate_id,
            e.payload,
            e.attempts
        "#,
    )
    .bind(batch_size)
    .bind(DEFAULT_LEASE_SECONDS)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(OutboxEvent::from).collect())
}

/// Mark an event as successfully processed.
pub async fn mark_processed(pool: &PgPool, event_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE outbox_events
        SET processed_at = NOW(),
            last_error = NULL
        WHERE id = $1
          AND processed_at IS NULL
        "#,
    )
    .bind(event_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Schedule an event for retry after `delay_seconds`.
///
/// The error is truncated to avoid storing an unexpectedly large error
/// message in the database.
pub async fn schedule_retry(
    pool: &PgPool,
    event_id: i64,
    delay_seconds: i64,
    error: &str,
) -> Result<(), sqlx::Error> {
    let delay_seconds = delay_seconds.max(1);
    let error: String = error.chars().take(4_000).collect();

    sqlx::query(
        r#"
        UPDATE outbox_events
        SET available_at =
                NOW() + ($2::BIGINT * INTERVAL '1 second'),
            last_error = $3
        WHERE id = $1
          AND processed_at IS NULL
        "#,
    )
    .bind(event_id)
    .bind(delay_seconds)
    .bind(error)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete successfully processed events older than the requested retention
/// period. Returns the number of rows deleted.
pub async fn cleanup_processed(pool: &PgPool, retention_days: i32) -> Result<u64, sqlx::Error> {
    let retention_days = retention_days.max(1);

    let result = sqlx::query(
        r#"
        DELETE FROM outbox_events
        WHERE processed_at IS NOT NULL
          AND processed_at < NOW() - ($1::INTEGER * INTERVAL '1 day')
        "#,
    )
    .bind(retention_days)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
