use sqlx::PgPool;

use crate::{Error::ApiResult, dto::tenant::CreateTenantRequest, model::tenant::Tenant};

pub async fn insert_tenant(tenant: &CreateTenantRequest, db_conn: &PgPool) -> ApiResult<Tenant> {
    let tenant = sqlx::query_as!(
        Tenant,
        r#"
        INSERT INTO tenants (name, slug)
        VALUES ($1, $2)
        RETURNING id, name, slug, status, created_at, updated_at
        "#,
        tenant.name,
        tenant.slug
    )
    .fetch_one(db_conn)
    .await?;

    Ok(tenant)
}
pub async fn list_tenant() {}

pub async fn get_tenant_by_id() {}

pub async fn updated_tenant() {}
