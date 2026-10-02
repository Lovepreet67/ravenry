use std::env;

use crate::error::ApiResult;
use crate::model::auth::Claims;
use jsonwebtoken::{DecodingKey, Validation, decode};

pub fn verify_token(token: &str) -> ApiResult<Claims> {
    let jwt_secret: String = env::var("JWT_SECRET").expect("Can't sign without JWT Secret");
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(), // default validates `exp` automatically
    )
    .map_err(|_| crate::error::ApiError::Gen("invalid or expired token"))?;
    Ok(data.claims)
}
