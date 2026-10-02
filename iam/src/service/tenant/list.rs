use sqlx::PgPool;

use crate::{
    dto::utils::{PaginatedList, Pagination},
    error::ApiResult,
    model::tenant::Tenant,
    service::database::tenant::list as list_db,
};

pub async fn list(pagination: &Pagination, db_conn: &PgPool) -> ApiResult<PaginatedList<Tenant>> {
    list_db(db_conn, pagination).await
}
