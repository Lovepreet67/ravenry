use sqlx::PgPool;

use crate::{
    Error::ApiResult,
    model::user::{NewUser, User},
};

pub async fn insert_user(new_user: &NewUser, db_conn: &PgPool) -> ApiResult<User> {
    let res = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users(username, email, full_name, password_hash)
        VALUES($1, $2, $3, $4)
        RETURNING id, username, email, full_name,password_hash, created_at, updated_at
        "#,
        new_user.username,
        new_user.email,
        new_user.full_name,
        new_user.password_hash,
    )
    .fetch_one(db_conn)
    .await?;
    Ok(res)
}
