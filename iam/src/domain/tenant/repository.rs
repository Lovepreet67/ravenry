use std::fmt::Debug;

use crate::{
    domain::tenant::dto::TenantFilter,
    domain::tenant::model::{NewTenant, Tenant},
    shared::{
        error::ApiResult,
        pagination::{PageInfo, PaginatedList, Pagination},
    },
};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait TenantRepository: Send + Sync + Debug {
    async fn create(&self, tenant: &NewTenant) -> ApiResult<Tenant>;
    async fn get(&self, id: Uuid) -> ApiResult<Option<Tenant>>;
    async fn get_by_slug(&self, slug: &str) -> ApiResult<Option<Tenant>>;
    async fn list(
        &self,
        filter: &TenantFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<Tenant>>;
    async fn update(&self, tenant: &Tenant) -> ApiResult<Tenant>;
}

#[derive(Debug, Clone)]
pub struct DatabaseTenantRepository {
    conn: PgPool,
}

impl DatabaseTenantRepository {
    pub fn new(conn: PgPool) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl TenantRepository for DatabaseTenantRepository {
    async fn create(&self, tenant: &NewTenant) -> ApiResult<Tenant> {
        let tenant = sqlx::query_as!(
            Tenant,
            r#" 
            INSERT INTO tenants (name, slug, default_role_id)
            VALUES ($1, $2, $3)
            RETURNING id, name, slug, is_active,default_role_id, created_at, updated_at
            "#,
            tenant.name,
            tenant.slug,
            tenant.default_role_id
        )
        .fetch_one(&self.conn)
        .await?; // sqlx::Error -> ApiError via the From impl above; unique violation on `slug` becomes 409 automatically

        Ok(tenant)
    }

    async fn get(&self, id: Uuid) -> ApiResult<Option<Tenant>> {
        let tenant = sqlx::query_as!(
            Tenant,
            r#"
            SELECT id, name, slug, is_active,default_role_id, created_at, updated_at
            FROM tenants
            WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.conn) // fetch_optional, not fetch_one — missing row is a valid, non-error outcome here
        .await?;

        Ok(tenant)
    }
    async fn get_by_slug(&self, slug: &str) -> ApiResult<Option<Tenant>> {
        let tenant = sqlx::query_as!(
            Tenant,
            r#"
            SELECT id, name, slug, is_active,default_role_id, created_at, updated_at
            FROM tenants
            WHERE slug = $1
            "#,
            slug,
        )
        .fetch_optional(&self.conn) // fetch_optional, not fetch_one — missing row is a valid, non-error outcome here
        .await?;

        Ok(tenant)
    }

    async fn list(
        &self,
        filter: &TenantFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<Tenant>> {
        let apply_filter = |qb: &mut QueryBuilder<Postgres>| {
            if let Some(is_active) = filter.is_active {
                qb.push(" WHERE is_active = ");
                qb.push_bind(is_active);
            }
        };

        let mut count_qb: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM tenants");
        apply_filter(&mut count_qb);
        let total_rows: i64 = count_qb.build_query_scalar().fetch_one(&self.conn).await?;

        let mut data_qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, name, slug, is_active,default_role_id, created_at, updated_at FROM tenants",
        );
        apply_filter(&mut data_qb);
        data_qb.push(" ORDER BY created_at DESC LIMIT ");
        data_qb.push_bind(pagination.get_page_size() as i64);
        data_qb.push(" OFFSET ");
        data_qb.push_bind(pagination.get_offset());

        let rows = data_qb
            .build_query_as::<Tenant>()
            .fetch_all(&self.conn)
            .await?;

        Ok(PaginatedList {
            list: rows,
            page_info: PageInfo {
                page_no: pagination.get_page_no(),
                page_size: pagination.get_page_size(),
                total_rows: total_rows,
            },
        })
    }

    async fn update(&self, tenant: &Tenant) -> ApiResult<Tenant> {
        let updated = sqlx::query_as!(
            Tenant,
            r#"
            UPDATE tenants
            SET name = $1, slug = $2, is_active = $3
            WHERE id = $4
            RETURNING id, name, slug, is_active,default_role_id, created_at, updated_at
            "#,
            tenant.name,
            tenant.slug,
            tenant.is_active,
            tenant.id,
        )
        .fetch_optional(&self.conn)
        .await?
        .ok_or(crate::shared::error::ApiError::NotFound)?; // no row matched `id` -> 404, not a silent panic

        Ok(updated)
    }
}
