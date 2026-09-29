use redis::{AsyncCommands, aio::ConnectionManager};

use crate::{Error::ApiResult, model::tenant::Tenant};
const TENANT_ACCOUNT_PREFIX: &str = "TENANT_ACCOUNT";

fn get_tenant_account_key(tenant: &Tenant) -> String {
    return format!("{}:{}", TENANT_ACCOUNT_PREFIX, &tenant.id);
}

pub async fn insert_tenant(tenant: &Tenant, conn: &mut ConnectionManager) -> ApiResult<()> {
    let key = get_tenant_account_key(&tenant);
    let inserted: bool = conn.hset_nx(&key, "id", tenant.id.to_string()).await?;
    if !inserted {
        return Err(crate::Error::ApiError::Gen("Tenant already exist"));
    }
    conn.hset_multiple::<_, _, _, ()>(
        &key,
        &[
            ("name", &tenant.name.clone().unwrap_or_else(|| "".into())),
            ("slug", &tenant.slug),
            ("created_at", &tenant.created_at.to_rfc3339()),
            ("updated_at", &tenant.updated_at.to_rfc3339()),
        ],
    )
    .await?;
    Ok(())
}

pub async fn update_tenant(tenant: &Tenant, conn: &mut ConnectionManager) -> ApiResult<()> {
    Ok(())
}
