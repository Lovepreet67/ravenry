use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::{
    Error::ApiResult,
    dto::tenant::CreateTenantRequest,
    model::tenant::Tenant,
    service::{
        database::tenant::insert_tenant as insert_tenant_db,
        redis::tenant::insert_tenant as insert_tenant_redis,
    },
};

pub async fn create_tenant(
    tenant_req: &CreateTenantRequest,
    db_conn: &mut PgPool,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<Tenant> {
    let tenant = insert_tenant_db(tenant_req, &db_conn).await?;
    if let Err(e) = insert_tenant_redis(&tenant, redis_conn).await {
        tracing::error!(
            "Error while inserting tenant {}, into redis {:?}",
            tenant.id,
            e
        );
    }
    Ok(tenant)
}
