use sqlx::PgPool;

use crate::{Error::ApiResult, dto::user::UserResponse};

pub async fn list(db_conn: &PgPool) -> ApiResult<Vec<UserResponse>> {
    let res = sqlx::query_as!(
        UserResponse,
        r#"
    SELECT id, username, full_name, email,created_at, updated_at From users
    "#
    )
    .fetch_all(db_conn)
    .await?;
    Ok(res)
}
