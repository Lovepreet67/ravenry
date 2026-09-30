use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::{
    Error::ApiResult,
    dto::{
        user::{UserFilter, UserResponse},
        utils::{PaginatedList, Pagination},
    },
    model::user::User,
    service::{
        database::user::list as list_db, redis::user::get_user_by_email as get_user_by_email_redis,
    },
};

pub async fn list(
    filter: &UserFilter,
    pagination: &Pagination,
    db_conn: &PgPool,
) -> ApiResult<PaginatedList<UserResponse>> {
    list_db(db_conn, filter, pagination).await
}

pub async fn get_by_email(
    email: &str,
    _db_conn: &PgPool,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<Option<User>> {
    get_user_by_email_redis(email, redis_conn).await
}
