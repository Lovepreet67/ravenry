use std::env;

use crate::{
    dto::auth::{AuthenticateResponse, LoginRequest, LoginResponse},
    error::ApiResult,
    model::user::User,
    service::{password::verify_password, user::list::get_by_email as get_user_by_email},
};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::model::auth::Claims;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

fn sign(user: &User) -> ApiResult<(String, usize)> {
    let jwt_secret: String = env::var("JWT_SECRET").expect("Can't sign without JWT Secret");
    let jwt_ttl_hours: i64 = env::var("JWT_TTL_HOURS")
        .unwrap_or("24".into())
        .parse()
        .expect("JWT_TTL_HOURS must be a valid number");
    let claims = Claims::new(user, jwt_ttl_hours);
    let exp = claims.exp;

    let token = encode(
        &Header::default(), // default = HS256
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| crate::error::ApiError::Gen("failed to sign token"))?;

    Ok((token, exp))
}

pub fn verify_token(token: &str, secret: &str) -> ApiResult<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(), // default validates `exp` automatically
    )
    .map_err(|_| crate::error::ApiError::Gen("invalid or expired token"))?;
    Ok(data.claims)
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
    // password is okk
    let (jwt, expires_at) = sign(&user)?;
    Ok(LoginResponse { jwt, expires_at })
}

pub async fn authenticate(
    token: &str,
    db_conn: &mut PgPool,
    redis_conn: &mut ConnectionManager,
) -> ApiResult<AuthenticateResponse> {
    Ok(AuthenticateResponse {
        user_id: Uuid::new_v4(),
        user_name: "TEsting".to_string(),
        tenant_id: Uuid::new_v4(),
    })
}
