use sqlx::{PgPool, QueryBuilder};

use crate::{
    Error::ApiResult,
    dto::{
        tenant::CreateTenantRequest,
        utils::{PageInfo, PaginatedList, Pagination},
    },
    model::tenant::Tenant,
};

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

pub async fn list(db_conn: &PgPool, pagination: &Pagination) -> ApiResult<PaginatedList<Tenant>> {
    // ---- total count, for page_info ----
    let mut count_qb: QueryBuilder<_> = QueryBuilder::new("SELECT COUNT(*) FROM users");
    let total_rows: i64 = count_qb.build_query_scalar().fetch_one(db_conn).await?;

    let mut data_qb: QueryBuilder<_> =
        QueryBuilder::new("SELECT id, name, slug, status, created_at, updated_at FROM tenants");
    data_qb.push(" ORDER BY created_at DESC LIMIT ");
    data_qb.push_bind(pagination.get_page_size());
    data_qb.push(" OFFSET ");
    data_qb.push_bind(pagination.get_page_size() * (pagination.get_page_no() - 1));

    let rows = data_qb
        .build_query_as::<Tenant>()
        .fetch_all(db_conn)
        .await?;

    Ok(PaginatedList {
        list: rows,
        page_info: PageInfo {
            page_size: pagination.get_page_size(),
            page_no: pagination.get_page_no(),
            total_rows: total_rows,
        },
    })
}
