use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{AttemptRepository, RepoResult};
use crate::db::map_err;
use crate::domain::TenantScope;
use crate::domain::delivery_attempt::DeliveryAttempt;

#[derive(Clone)]
pub struct PgAttemptRepository {
    pool: PgPool,
}

impl PgAttemptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AttemptRow {
    attempt_id: Uuid,
    delivery_id: Uuid,
    attempt_number: i32,
    success: bool,
    status_code: Option<i32>,
    error: Option<String>,
    attempted_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
}

impl From<AttemptRow> for DeliveryAttempt {
    fn from(r: AttemptRow) -> Self {
        Self {
            attempt_id: r.attempt_id,
            delivery_id: r.delivery_id,
            attempt_number: r.attempt_number,
            success: r.success,
            status_code: r.status_code,
            error: r.error,
            attempted_at: r.attempted_at,
            recorded_at: r.recorded_at,
        }
    }
}

#[async_trait]
impl AttemptRepository for PgAttemptRepository {
    async fn list_for_delivery(
        &self,
        scope: &TenantScope,
        delivery_id: Uuid,
    ) -> RepoResult<Vec<DeliveryAttempt>> {
        // `delivery_attempts` has no tenant column, so ownership is
        // enforced by joining to `deliveries`. A delivery owned by another
        // tenant matches no row, exactly like a missing one.
        let rows: Vec<AttemptRow> = sqlx::query_as(
            r#"
            SELECT a.attempt_id, a.delivery_id, a.attempt_number, a.success,
                   a.status_code, a.error, a.attempted_at, a.recorded_at
              FROM delivery_attempts a
              JOIN deliveries d ON d.delivery_id = a.delivery_id
             WHERE d.tenant_id = $1
               AND a.delivery_id = $2
             ORDER BY a.attempt_number DESC
            "#,
        )
        .bind(scope.tenant_id())
        .bind(delivery_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}
