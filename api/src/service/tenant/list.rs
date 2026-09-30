use sqlx::PgPool;

use crate::{
    Error::ApiResult,
    dto::utils::{PaginatedList, Pagination},
    model::tenant::Tenant,
    service::database::tenant::list as list_db,
};

pub async fn list(pagination: &Pagination, db_conn: &PgPool) -> ApiResult<PaginatedList<Tenant>> {
    list_db(db_conn, pagination).await
}
