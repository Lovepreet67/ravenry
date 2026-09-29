use redis::{AsyncCommands, aio::ConnectionManager};

use crate::{Error::ApiResult, model::user::User};
const USER_ACCOUNT_PREFIX: &str = "USER_ACCOUNT";

fn get_user_account_key(user: &User) -> String {
    return format!("{}:{}", USER_ACCOUNT_PREFIX, &user.id);
}

pub async fn insert_user(user: &User, conn: &mut ConnectionManager) -> ApiResult<()> {
    let key = get_user_account_key(&user);
    let inserted: bool = conn.hset_nx(&key, "id", user.id.to_string()).await?;
    if !inserted {
        return Err(crate::Error::ApiError::Gen("Tenant already exist"));
    }
    conn.hset_multiple::<_, _, _, ()>(
        &key,
        &[
            ("full_name", &user.full_name),
            ("email", &user.email),
            ("password_hash", &user.password_hash),
            ("username", &user.username),
            ("created_at", &user.created_at.to_rfc3339()),
            ("updated_at", &user.updated_at.to_rfc3339()),
        ],
    )
    .await?;
    Ok(())
}

pub async fn update_user(tenant: &User, conn: &mut ConnectionManager) -> ApiResult<()> {
    Ok(())
}
