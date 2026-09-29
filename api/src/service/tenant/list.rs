use sqlx::PgPool;

use crate::{Error::ApiResult, dto::tenant::TenantResponse};

pub async fn list(db_conn: &PgPool) -> ApiResult<Vec<TenantResponse>> {
    let res = sqlx::query_as!(
        TenantResponse,
        r#"
        SELECT id,name,slug,created_at,updated_at FROM tenants;
        "#
    )
    .fetch_all(db_conn)
    .await?;
    Ok(res)
}
