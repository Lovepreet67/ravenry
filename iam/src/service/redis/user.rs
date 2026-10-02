use std::{collections::HashMap, format};

use chrono::Utc;
use redis::{AsyncTypedCommands, aio::ConnectionManager};

use crate::{
    dto::user::UpsertMemebershipRequest,
    error::{ApiError, ApiResult},
    model::user::{User, UserMemberships},
};
const USER_ACCOUNT_PREFIX: &str = "USER_ACCOUNT";
const USER_ACCOUNT_BY_EMAIL: &str = "USER_ACCOUNT_BY_EMAIL";
const USER_ACCOUNT_BY_USERNAME: &str = "USER_ACCOUNT_BY_USERNAME";
const USER_MEMBERSHIPS_BY_USER_ID: &str = "USER_MEMBERSHIPS_BY_USER_ID";

fn get_user_membership_key(user_id: impl std::fmt::Display) -> String {
    format!("{USER_MEMBERSHIPS_BY_USER_ID}:{}", user_id)
}

fn get_user_key(id: impl std::fmt::Display) -> String {
    format!("{USER_ACCOUNT_PREFIX}:{id}")
}

fn get_user_email_key(email: &str) -> String {
    format!("{}:{}", USER_ACCOUNT_BY_EMAIL, email)
}

fn get_user_username_key(username: &str) -> String {
    format!("{}:{}", USER_ACCOUNT_BY_USERNAME, username)
}

pub async fn insert_user(user: &User, conn: &mut ConnectionManager) -> ApiResult<()> {
    let user_key = get_user_key(&user.id);
    let email_key = get_user_email_key(&user.email);
    let username_key = get_user_username_key(&user.username);

    // Canonical user
    conn.hset_multiple(
        &user_key,
        &[
            ("id", user.id.to_string()),
            ("full_name", user.full_name.clone()),
            ("email", user.email.clone()),
            ("password_hash", user.password_hash.clone()),
            ("username", user.username.clone()),
            ("created_at", user.created_at.to_rfc3339()),
            ("updated_at", user.updated_at.to_rfc3339()),
        ],
    )
    .await?;

    // Lookup indexes
    conn.set(&email_key, user.id.to_string()).await?;
    conn.set(&username_key, user.id.to_string()).await?;

    Ok(())
}

async fn load_user_by_id(user_id: &str, conn: &mut ConnectionManager) -> ApiResult<Option<User>> {
    let fields: HashMap<String, String> = conn.hgetall(get_user_key(user_id)).await?;
    if fields.is_empty() {
        return Ok(None);
    }
    Ok(Some(User {
        id: fields["id"].parse().map_err(|_| ApiError::Gen("bad id"))?,
        full_name: fields.get("full_name").cloned().unwrap_or_default(),
        email: fields.get("email").cloned().unwrap_or_default(),
        username: fields.get("username").cloned().unwrap_or_default(),
        password_hash: fields.get("password_hash").cloned().unwrap_or_default(),
        created_at: parse_dt(&fields, "created_at")?,
        updated_at: parse_dt(&fields, "updated_at")?,
    }))
}

fn parse_dt(fields: &HashMap<String, String>, key: &str) -> ApiResult<chrono::DateTime<Utc>> {
    fields
        .get(key)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or(ApiError::Gen("bad timestamp"))
}

pub async fn get_user_by_email(
    email: &str,
    conn: &mut ConnectionManager,
) -> ApiResult<Option<User>> {
    let user_id: Option<String> = conn.get(get_user_email_key(email)).await?;
    match user_id {
        Some(id) => load_user_by_id(&id, conn).await,
        None => Ok(None),
    }
}

pub async fn get_user_by_username(
    username: &str,
    conn: &mut ConnectionManager,
) -> ApiResult<Option<User>> {
    let user_id: Option<String> = conn.get(get_user_username_key(username)).await?;
    match user_id {
        Some(id) => load_user_by_id(&id, conn).await,
        None => Ok(None),
    }
}

pub async fn upsert_membership(
    req: &UpsertMemebershipRequest,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<()> {
    let key = get_user_membership_key(&req.user_id);
    redis_conn
        .hset(&key, &req.tenant_id.to_string(), &req.role_id.to_string())
        .await?;
    Ok(())
}

pub async fn list_memberships(
    user_id: impl std::fmt::Display,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<UserMemberships> {
    let key = get_user_membership_key(&user_id);
    let x = redis_conn.hgetall(&key).await?;
    let memberships = UserMemberships::try_from(x)?;
    Ok(memberships)
}
