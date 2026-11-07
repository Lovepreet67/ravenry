use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::{EventRepository, RepoResult};
use crate::db::map_err;
use crate::domain::TenantScope;
use crate::domain::event::{EventDetail, EventFilter, EventSummary};
use crate::domain::pagination::{Cursor, Page, PageRequest};

#[derive(Clone)]
pub struct PgEventRepository {
    pool: PgPool,
}

impl PgEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    event_id: Uuid,
    topic_id: Uuid,
    topic_path: String,
    schema_version: i32,
    received_at: DateTime<Utc>,
}

impl From<SummaryRow> for EventSummary {
    fn from(r: SummaryRow) -> Self {
        Self {
            event_id: r.event_id,
            topic_id: r.topic_id,
            topic_path: r.topic_path,
            schema_version: r.schema_version,
            received_at: r.received_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct DetailRow {
    #[sqlx(flatten)]
    summary: SummaryRow,
    payload: serde_json::Value,
}

#[async_trait]
impl EventRepository for PgEventRepository {
    async fn list(
        &self,
        scope: &TenantScope,
        filter: &EventFilter,
        page: &PageRequest,
    ) -> RepoResult<Page<EventSummary>> {
        // The payload column is never selected here, so large payloads
        // are never read for list views.
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT event_id, topic_id, topic_path, schema_version, received_at \
             FROM events WHERE tenant_id = ",
        );
        qb.push_bind(scope.tenant_id());

        if let Some(topic_id) = filter.topic_id {
            qb.push(" AND topic_id = ").push_bind(topic_id);
        }
        if let Some(path) = &filter.topic_path {
            qb.push(" AND topic_path = ").push_bind(path.clone());
        }
        if let Some(from) = filter.from {
            qb.push(" AND received_at >= ").push_bind(from);
        }
        if let Some(to) = filter.to {
            qb.push(" AND received_at < ").push_bind(to);
        }
        if let Some(c) = &page.cursor {
            qb.push(" AND (received_at, event_id) < (")
                .push_bind(c.ts)
                .push(", ")
                .push_bind(c.id)
                .push(")");
        }

        qb.push(" ORDER BY received_at DESC, event_id DESC LIMIT ")
            .push_bind(page.fetch_limit());

        let rows: Vec<SummaryRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        let items: Vec<EventSummary> = rows.into_iter().map(Into::into).collect();
        Ok(Page::from_fetched(items, page.limit, |e| Cursor {
            ts: e.received_at,
            id: e.event_id,
        }))
    }

    async fn get(&self, scope: &TenantScope, event_id: Uuid) -> RepoResult<Option<EventDetail>> {
        let row: Option<DetailRow> = sqlx::query_as(
            "SELECT event_id, topic_id, topic_path, schema_version, received_at, payload \
             FROM events WHERE tenant_id = $1 AND event_id = $2",
        )
        .bind(scope.tenant_id())
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(|r| EventDetail {
            summary: r.summary.into(),
            payload: r.payload,
        }))
    }
}
