use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::{DeliveryRepository, RepoResult};
use crate::db::map_err;
use crate::domain::TenantScope;
use crate::domain::delivery::{Delivery, DeliveryFilter, DeliveryStatus};
use crate::domain::pagination::{Cursor, Page, PageRequest};
use crate::error::RepoError;

#[derive(Clone)]
pub struct PgDeliveryRepository {
    pool: PgPool,
}

impl PgDeliveryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const COLUMNS: &str = "delivery_id, event_id, topic_id, destination_id, status, \
                       current_attempt, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct DeliveryRow {
    delivery_id: Uuid,
    event_id: Uuid,
    topic_id: Uuid,
    destination_id: Uuid,
    status: String,
    current_attempt: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<DeliveryRow> for Delivery {
    type Error = RepoError;

    fn try_from(r: DeliveryRow) -> Result<Self, RepoError> {
        let status = DeliveryStatus::parse(&r.status)
            .ok_or_else(|| RepoError::Backend(format!("unknown delivery status '{}'", r.status)))?;
        Ok(Self {
            delivery_id: r.delivery_id,
            event_id: r.event_id,
            topic_id: r.topic_id,
            destination_id: r.destination_id,
            status,
            current_attempt: r.current_attempt,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

#[async_trait]
impl DeliveryRepository for PgDeliveryRepository {
    async fn list(
        &self,
        scope: &TenantScope,
        filter: &DeliveryFilter,
        page: &PageRequest,
    ) -> RepoResult<Page<Delivery>> {
        let mut qb = QueryBuilder::<Postgres>::new(format!(
            "SELECT {COLUMNS} FROM deliveries WHERE tenant_id = "
        ));
        qb.push_bind(scope.tenant_id());

        if let Some(event_id) = filter.event_id {
            qb.push(" AND event_id = ").push_bind(event_id);
        }
        if let Some(dest) = filter.destination_id {
            qb.push(" AND destination_id = ").push_bind(dest);
        }
        if let Some(status) = filter.status {
            qb.push(" AND status = ").push_bind(status.as_str());
        }
        if let Some(from) = filter.from {
            qb.push(" AND created_at >= ").push_bind(from);
        }
        if let Some(to) = filter.to {
            qb.push(" AND created_at < ").push_bind(to);
        }
        if let Some(c) = &page.cursor {
            qb.push(" AND (created_at, delivery_id) < (")
                .push_bind(c.ts)
                .push(", ")
                .push_bind(c.id)
                .push(")");
        }

        qb.push(" ORDER BY created_at DESC, delivery_id DESC LIMIT ")
            .push_bind(page.fetch_limit());

        let rows: Vec<DeliveryRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        let items = rows
            .into_iter()
            .map(Delivery::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Page::from_fetched(items, page.limit, |d| Cursor {
            ts: d.created_at,
            id: d.delivery_id,
        }))
    }

    async fn get(&self, scope: &TenantScope, delivery_id: Uuid) -> RepoResult<Option<Delivery>> {
        let row: Option<DeliveryRow> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM deliveries WHERE tenant_id = $1 AND delivery_id = $2"
        ))
        .bind(scope.tenant_id())
        .bind(delivery_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        row.map(Delivery::try_from).transpose()
    }
}
