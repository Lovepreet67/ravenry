use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, QueryBuilder};

use super::model::{InboxMessage, NewInboxMessage};
use super::store::InboxStore;
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct PgInboxStore {
    pool: PgPool,
}

impl PgInboxStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InboxStore for PgInboxStore {
    async fn insert_batch(&self, messages: &[NewInboxMessage]) -> Result<u64> {
        if messages.is_empty() {
            return Ok(0);
        }

        let mut qb = QueryBuilder::new(
            "INSERT INTO inbox_messages (
                source, stream_name, queue_message_id, message_type,
                schema_version, payload, raw_payload, status, last_error
            ) ",
        );

        qb.push_values(messages, |mut row, m| {
            row.push_bind(&m.source)
                .push_bind(&m.stream_name)
                .push_bind(&m.queue_message_id)
                .push_bind(&m.message_type)
                .push_bind(m.schema_version)
                .push_bind(&m.payload)
                .push_bind(&m.raw_payload)
                .push_bind(m.status.as_str())
                .push_bind(&m.last_error);
        });
        qb.push(" ON CONFLICT (stream_name, queue_message_id) DO NOTHING");

        // A single statement is atomic, so no explicit transaction is needed.
        let res = qb.build().execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    async fn claim_batch(
        &self,
        source: &str,
        worker_id: &str,
        limit: i64,
        lease: Duration,
    ) -> Result<Vec<InboxMessage>> {
        let mut rows: Vec<InboxMessage> = sqlx::query_as(
            r#"
            UPDATE inbox_messages
               SET status       = 'processing',
                   locked_by    = $2,
                   locked_until = NOW() + make_interval(secs => $3)
             WHERE id IN (
                   SELECT id
                     FROM inbox_messages
                    WHERE source = $1
                      AND status = 'pending'
                      AND available_at <= NOW()
                    ORDER BY available_at, id
                    LIMIT $4
                      FOR UPDATE SKIP LOCKED
             )
            RETURNING id, source, stream_name, queue_message_id, message_type,
                      schema_version, payload, attempts, deferrals, received_at
            "#,
        )
        .bind(source)
        .bind(worker_id)
        .bind(lease.as_secs_f64())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        // RETURNING order is not guaranteed.
        rows.sort_by_key(|r| r.id);
        Ok(rows)
    }

    async fn schedule_retry(
        &self,
        id: i64,
        error: &str,
        available_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE inbox_messages
               SET status = 'pending', attempts = attempts + 1,
                   available_at = $2, last_error = $3,
                   locked_by = NULL, locked_until = NULL
             WHERE id = $1 AND status = 'processing'
            "#,
        )
        .bind(id)
        .bind(available_at)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn defer(&self, id: i64, reason: &str, available_at: DateTime<Utc>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE inbox_messages
               SET status = 'pending', deferrals = deferrals + 1,
                   available_at = $2, last_error = $3,
                   locked_by = NULL, locked_until = NULL
             WHERE id = $1 AND status = 'processing'
            "#,
        )
        .bind(id)
        .bind(available_at)
        .bind(reason)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_failed(&self, id: i64, error: &str) -> Result<()> {
        self.finish(id, "failed", error).await
    }

    async fn mark_quarantined(&self, id: i64, error: &str) -> Result<()> {
        self.finish(id, "quarantined", error).await
    }

    async fn reap_stale(&self, max_attempts: i32) -> Result<u64> {
        let res = sqlx::query(
            r#"
            UPDATE inbox_messages
               SET attempts     = attempts + 1,
                   status       = CASE WHEN attempts + 1 >= $1
                                       THEN 'failed' ELSE 'pending' END,
                   last_error   = 'lease expired',
                   locked_by    = NULL,
                   locked_until = NULL
             WHERE status = 'processing' AND locked_until < NOW()
            "#,
        )
        .bind(max_attempts)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }
}

impl PgInboxStore {
    async fn finish(&self, id: i64, status: &str, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE inbox_messages
               SET status = $2, last_error = $3, processed_at = NOW(),
                   locked_by = NULL, locked_until = NULL
             WHERE id = $1 AND status = 'processing'
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// Mark a row processed INSIDE the caller's transaction, so the business
/// write and the inbox completion commit (or roll back) together.
///
/// Fails with `LeaseLost` if the lease expired and the row was reclaimed.
/// The caller then rolls back, and the new owner processes it. Business
/// writes are idempotent, so this is safe.
pub async fn mark_processed_tx(conn: &mut PgConnection, id: i64) -> Result<()> {
    let res = sqlx::query(
        r#"
        UPDATE inbox_messages
           SET status = 'processed', processed_at = NOW(),
               locked_by = NULL, locked_until = NULL
         WHERE id = $1 AND status = 'processing'
        "#,
    )
    .bind(id)
    .execute(conn)
    .await?;

    if res.rows_affected() == 0 {
        return Err(Error::LeaseLost(id));
    }
    Ok(())
}