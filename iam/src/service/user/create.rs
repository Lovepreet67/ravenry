use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::{
    error::ApiResult,
    dto::user::{CreateUserRequest, UserResponse},
    model::user::NewUser,
    service::{
        database::user::insert_user as insert_user_db,
        redis::user::insert_user as insert_user_redis,
    },
};

pub async fn create_user(
    user_rq: CreateUserRequest,
    db_conn: &mut PgPool,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<UserResponse> {
    let new_user: NewUser = user_rq.into();
    let user = insert_user_db(&new_user, db_conn).await?;
    if let Err(e) = insert_user_redis(&user, redis_conn).await {
        tracing::error!(
            "Error while inserting tenant {}, into redis {:?}",
            user.id,
            e
        );
    }
    Ok(user.into())
}
