use std::{fmt::Debug, println};

use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::{
    domain::topic::{
        dto::TopicFilter,
        model::{DestinationLink, NewTopic, Topic},
    },
    platform::outbox::{event::NewOutboxEvent, store as outbox_store},
    shared::{
        error::ApiResult,
        pagination::{PageInfo, PaginatedList, Pagination},
    },
};

#[async_trait::async_trait]
pub trait TopicRepository: Send + Sync + Debug {
    async fn list(
        &self,
        tenant_id: uuid::Uuid,
        filter: &TopicFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<Topic>>;
    async fn create(&self, topic: NewTopic) -> ApiResult<Topic>;
    async fn update(&self, topic: Topic) -> ApiResult<Topic>;
    async fn get_by_id(
        &self,
        topic_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
    ) -> ApiResult<Option<Topic>>;
    async fn put_destination(&self, destination_link: DestinationLink) -> ApiResult<Topic>;
    async fn delete_destination(&self, destination_link: DestinationLink) -> ApiResult<Topic>;
}

#[derive(Debug)]
pub struct DatabaseTopicRepository {
    db_conn: PgPool,
}

impl DatabaseTopicRepository {
    pub fn new(db_conn: PgPool) -> Self {
        Self { db_conn }
    }
    async fn valid_linking(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        destination_link: &DestinationLink,
    ) -> ApiResult<bool> {
        let valid: bool = sqlx::query_scalar(
            r#"
        SELECT EXISTS (
            SELECT 1
            FROM topics t
            JOIN destinations d
              ON d.tenant_id = t.tenant_id
            WHERE t.id = $1
              AND d.id = $2
              AND t.tenant_id = $3
        )
        "#,
        )
        .bind(destination_link.topic_id)
        .bind(destination_link.destination_id)
        .bind(destination_link.tenant_id)
        .fetch_one(&mut **tx)
        .await?;
        Ok(valid)
    }
}

fn apply_filter<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    tenant_id: uuid::Uuid,
    filter: &'a TopicFilter,
) {
    qb.push(" WHERE tenant_id = ").push_bind(tenant_id);
    if let Some(name) = filter.name.as_ref() {
        qb.push(" AND name ILIKE ").push_bind(format!("%{name}%"));
    }
}

#[async_trait::async_trait]
impl TopicRepository for DatabaseTopicRepository {
    async fn create(&self, topic: NewTopic) -> ApiResult<Topic> {
        let mut tx = self.db_conn.begin().await?;
        let created = sqlx::query_as!(
            Topic,
            r#"INSERT INTO topics
                (name, description, path, tenant_id, created_by, last_updated_by)
            VALUES ($1, $2, $3, $4, $5, $5)
            RETURNING *
            "#,
            topic.name,
            topic.description,
            topic.path,
            topic.tenant_id,
            topic.created_by,
        )
        .fetch_one(&mut *tx)
        .await?;
        let event = NewOutboxEvent::tenant_projection_refresh(topic.tenant_id);
        outbox_store::insert(&mut tx, &event).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn get_by_id(
        &self,
        topic_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
    ) -> ApiResult<Option<Topic>> {
        let destination = sqlx::query_as!(
            Topic,
            r#"SELECT * FROM topics
            WHERE tenant_id = $1 AND id = $2
            "#,
            tenant_id,
            topic_id
        )
        .fetch_optional(&self.db_conn)
        .await?;
        Ok(destination)
    }

    async fn list(
        &self,
        tenant_id: uuid::Uuid,
        filter: &TopicFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<Topic>> {
        let mut count_qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM topics");
        apply_filter(&mut count_qb, tenant_id, filter);
        let total_rows: i64 = count_qb
            .build_query_scalar()
            .fetch_one(&self.db_conn)
            .await?;

        let mut data_qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM topics");
        apply_filter(&mut data_qb, tenant_id, filter);
        data_qb.push(" ORDER BY created_at DESC LIMIT ");
        data_qb.push_bind(pagination.get_page_size() as i64);
        data_qb.push(" OFFSET ");
        data_qb.push_bind(pagination.get_offset() as i64);

        let rows = data_qb
            .build_query_as::<Topic>()
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

    async fn update(&self, topic: Topic) -> ApiResult<Topic> {
        let mut tx = self.db_conn.begin().await?;
        let updated = sqlx::query_as!(
            Topic,
            r#"UPDATE topics
            SET name = $1, description = $2, is_active = $3, last_updated_by = $4
            WHERE id = $5 AND tenant_id = $6
            RETURNING *
            "#,
            topic.name,
            topic.description,
            topic.is_active,
            topic.last_updated_by,
            topic.id,
            topic.tenant_id,
        )
        .fetch_one(&mut *tx)
        .await?;
        let event = NewOutboxEvent::tenant_projection_refresh(topic.tenant_id);
        outbox_store::insert(&mut tx, &event).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn put_destination(&self, destination_link: DestinationLink) -> ApiResult<Topic> {
        println!("Putting");
        let mut tx = self.db_conn.begin().await?;
        let valid = Self::valid_linking(&mut tx, &destination_link).await?;

        if !valid {
            return Err(crate::shared::error::ApiError::NotFound);
        }

        // Attach the destination. Repeated PUT requests are safe.
        sqlx::query(
            r#"
        INSERT INTO topics_junc_destinations
            (topic_id, destination_id)
        VALUES ($1, $2)
        ON CONFLICT (topic_id, destination_id) DO NOTHING
        "#,
        )
        .bind(destination_link.topic_id)
        .bind(destination_link.destination_id)
        // .bind(&destination_link.created_by)
        .execute(&mut *tx)
        .await?;

        // Return the topic belonging to this tenant.
        let topic = sqlx::query_as!(
            Topic,
            r#"
        SELECT *
        FROM topics
        WHERE id = $1 AND tenant_id = $2
        "#,
            destination_link.topic_id,
            destination_link.tenant_id,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(crate::shared::error::ApiError::NotFound)?;
        let event = NewOutboxEvent::tenant_projection_refresh(destination_link.tenant_id);
        outbox_store::insert(&mut tx, &event).await?;
        tx.commit().await?;

        Ok(topic)
    }
    async fn delete_destination(&self, destination_link: DestinationLink) -> ApiResult<Topic> {
        let mut tx = self.db_conn.begin().await?;

        // Ensure both resources belong to the requested tenant.
        let valid = Self::valid_linking(&mut tx, &destination_link).await?;

        if !valid {
            return Err(crate::shared::error::ApiError::NotFound);
        }

        // Detach the destination; do not delete the destination itself.
        let deleted = sqlx::query(
            r#"
        DELETE FROM topics_junc_destinations
        WHERE topic_id = $1
          AND destination_id = $2
        "#,
        )
        .bind(destination_link.topic_id)
        .bind(destination_link.destination_id)
        .execute(&mut *tx)
        .await?;

        if deleted.rows_affected() == 0 {
            return Err(crate::shared::error::ApiError::NotFound);
        }

        let topic = sqlx::query_as!(
            Topic,
            r#"
        SELECT *
        FROM topics
        WHERE id = $1 AND tenant_id = $2
        "#,
            destination_link.topic_id,
            destination_link.tenant_id,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(crate::shared::error::ApiError::NotFound)?;
        let event = NewOutboxEvent::tenant_projection_refresh(destination_link.tenant_id);
        outbox_store::insert(&mut tx, &event).await?;

        tx.commit().await?;

        Ok(topic)
    }
}
