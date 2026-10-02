use redis::aio::ConnectionManager;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    dto::user::{UpsertMemebershipRequest, UpsertMemebershipResponse},
    error::ApiResult,
    model::user::UserMemberships,
    service::{
        database::user::upsert_membership as upsert_membership_db,
        redis::user::{list_memberships, upsert_membership as upsert_mebership_redis},
    },
};

pub async fn create_membership(
    req: &UpsertMemebershipRequest,
    db_conn: &mut PgPool,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<UpsertMemebershipResponse> {
    let created = upsert_membership_db(req, db_conn).await?;
    if let Err(e) = upsert_mebership_redis(req, redis_conn).await {
        tracing::error!(
            "Error while inserting membership {}, into redis {:?}",
            req.role_id,
            e
        );
    }
    Ok(UpsertMemebershipResponse::from_request(req, created))
}

pub async fn get_memberships(
    user_id: &Uuid,
    _db_conn: &mut PgPool,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<UserMemberships> {
    list_memberships(user_id, redis_conn).await
}
