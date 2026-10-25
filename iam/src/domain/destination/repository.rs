use std::{fmt::Debug, println};

use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::{
    domain::destination::{
        common::DestinationType,
        dto::DestinationFilter,
        model::{Destination, NewDestination},
    },
    platform::outbox::{event::NewOutboxEvent, store as outbox_store},
    shared::{
        error::ApiResult,
        pagination::{PageInfo, PaginatedList, Pagination},
    },
};

#[async_trait::async_trait]
pub trait DestinationRepository: Send + Sync + Debug {
    async fn list(
        &self,
        tenant_id: uuid::Uuid,
        filter: &DestinationFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<Destination>>;
    async fn create(&self, destination: NewDestination) -> ApiResult<Destination>;
    async fn update(&self, destination: Destination) -> ApiResult<Destination>;
    async fn get_by_id(
        &self,
        destination_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
    ) -> ApiResult<Option<Destination>>;
}

#[derive(Debug)]
pub struct DatabaseDestinationRepository {
    db_conn: PgPool,
}

impl DatabaseDestinationRepository {
    pub fn new(db_conn: PgPool) -> Self {
        Self { db_conn }
    }
}

fn apply_filter<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    tenant_id: uuid::Uuid,
    filter: &'a DestinationFilter,
) {
    println!("Applying filter");
    qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
    if let Some(name) = filter.name.as_ref() {
        qb.push(" AND name ILIKE ").push_bind(format!("%{name}%"));
    }
    if let Some(kind) = filter.kind {
        qb.push(" AND kind = ").push_bind(kind);
    }
    if let Some(topic_id) = filter.topic_id {
        let linked = filter.linked.unwrap_or(true);
        println!("topicid {}, {}", topic_id, linked);
        if linked {
            qb.push(
            " AND id IN (SELECT destination_id from topics_junc_destinations where topic_id  = ",
        );
        } else {
            qb.push(
            " AND id NOT IN (SELECT destination_id from topics_junc_destinations where topic_id  = ",
        );
        }
        qb.push_bind(topic_id);
        qb.push(" )");
    }
}

#[async_trait::async_trait]
impl DestinationRepository for DatabaseDestinationRepository {
    async fn create(&self, destination: NewDestination) -> ApiResult<Destination> {
        let mut tx = self.db_conn.begin().await?;
        let created = sqlx::query_as!(
            Destination,
            r#"INSERT INTO destinations
                (name, kind, description, destination_url, delivery_timeout_seconds,
                 tenant_id, created_by, last_updated_by)
            VALUES ($1, $2::destination_kind, $3, $4, $5, $6, $7, $7)
            RETURNING id, name, kind as "kind: DestinationType", description,
                destination_url, delivery_timeout_seconds, is_active, tenant_id,
                created_by, last_updated_by, created_at, updated_at
            "#,
            destination.name,
            destination.kind as DestinationType,
            destination.description,
            destination.destination_url,
            destination.delivery_timeout_seconds,
            destination.tenant_id,
            destination.created_by,
        )
        .fetch_one(&mut *tx)
        .await?;
        let event = NewOutboxEvent::tenant_projection_refresh(destination.tenant_id);
        outbox_store::insert(&mut tx, &event).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn get_by_id(
        &self,
        destination_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
    ) -> ApiResult<Option<Destination>> {
        let destination = sqlx::query_as!(
            Destination,
            r#"SELECT id, name, kind as "kind: DestinationType", description,
                destination_url, delivery_timeout_seconds, is_active, tenant_id,
                created_by, last_updated_by, created_at, updated_at
            FROM destinations
            WHERE tenant_id = $1 AND id = $2
            "#,
            tenant_id,
            destination_id
        )
        .fetch_optional(&self.db_conn)
        .await?;
        Ok(destination)
    }

    async fn list(
        &self,
        tenant_id: uuid::Uuid,
        filter: &DestinationFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<Destination>> {
        let mut count_qb: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM destinations");
        apply_filter(&mut count_qb, tenant_id, filter);
        let total_rows: i64 = count_qb
            .build_query_scalar()
            .fetch_one(&self.db_conn)
            .await?;

        let mut data_qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, name, kind, description, destination_url, \
             delivery_timeout_seconds, is_active, tenant_id, created_by, \
             last_updated_by, created_at, updated_at \
             FROM destinations",
        );
        apply_filter(&mut data_qb, tenant_id, filter);
        data_qb.push(" ORDER BY created_at DESC LIMIT ");
        data_qb.push_bind(pagination.get_page_size() as i64);
        data_qb.push(" OFFSET ");
        data_qb.push_bind(pagination.get_offset() as i64);

        let rows = data_qb
            .build_query_as::<Destination>()
            .fetch_all(&self.db_conn)
            .await?;

        Ok(PaginatedList {
            list: rows,
            page_info: PageInfo {
                page_no: pagination.get_page_no(),
                page_size: pagination.get_page_size(),
                total_rows,
            },
        })
    }

    async fn update(&self, destination: Destination) -> ApiResult<Destination> {
        let mut tx = self.db_conn.begin().await?;
        let updated = sqlx::query_as!(
            Destination,
            r#"UPDATE destinations
            SET name = $1, description = $2, kind = $3::destination_kind,
                destination_url = $4, is_active = $5, delivery_timeout_seconds = $6,
                last_updated_by = $7, updated_at = now()
            WHERE id = $8 AND tenant_id = $9
            RETURNING id, name, kind as "kind: DestinationType", description,
                destination_url, delivery_timeout_seconds, is_active, tenant_id,
                created_by, last_updated_by, created_at, updated_at
            "#,
            destination.name,
            destination.description,
            destination.kind as DestinationType,
            destination.destination_url,
            destination.is_active,
            destination.delivery_timeout_seconds,
            destination.last_updated_by,
            destination.id,
            destination.tenant_id,
        )
        .fetch_one(&mut *tx)
        .await?;
        let event = NewOutboxEvent::tenant_projection_refresh(destination.tenant_id);
        outbox_store::insert(&mut tx, &event).await?;
        tx.commit().await?;
        Ok(updated)
    }
}
