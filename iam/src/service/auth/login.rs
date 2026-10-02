use std::env;

use crate::{
    dto::auth::{AuthenticateResponse, LoginRequest, LoginResponse},
    error::ApiResult,
    model::user::{User, UserMemberships},
    service::{
        password::verify_password,
        user::{list::get_by_email as get_user_by_email, membership::get_memberships},
    },
};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::model::auth::Claims;
use jsonwebtoken::{EncodingKey, Header, encode};

fn sign(user: &User, memberhips: UserMemberships) -> ApiResult<(String, usize)> {
    let jwt_secret: String = env::var("JWT_SECRET").expect("Can't sign without JWT Secret");
    let jwt_ttl_hours: i64 = env::var("JWT_TTL_HOURS")
        .unwrap_or("24".into())
        .parse()
        .expect("JWT_TTL_HOURS must be a valid number");
    let claims = Claims::new(user, memberhips, jwt_ttl_hours);
    let exp = claims.exp;

    let token = encode(
        &Header::default(), // default = HS256
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| crate::error::ApiError::Gen("failed to sign token"))?;

    Ok((token, exp))
}

pub async fn login(
    req: &LoginRequest,
    db_conn: &mut PgPool,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<LoginResponse> {
    req.validate()?;
    let user = if let Some(username) = req.username.as_ref() {
        get_user_by_email(username, db_conn, redis_conn).await?
    } else if let Some(email) = req.email.as_ref() {
        get_user_by_email(email, db_conn, redis_conn).await?
    } else {
        return Err(crate::error::ApiError::Gen(
            "Both Username and email is missing",
        ));
    };
    let Some(user) = user else {
        return Err(crate::error::ApiError::Gen("Invalid Username or Password"));
    };
    verify_password(&req.password, &user.password_hash)?;
    let memberships = get_memberships(&user.id, db_conn, redis_conn).await?;
    // password is okk
    let (jwt, expires_at) = sign(&user, memberships)?;
    Ok(LoginResponse { jwt, expires_at })
}
