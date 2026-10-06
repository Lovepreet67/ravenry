use std::env;

use crate::error::ApiResult;
use crate::model::auth::Claims;
use jsonwebtoken::{DecodingKey, Validation, decode};

pub fn verify_token(token: &str) -> ApiResult<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_rsa_pem(include_bytes!("../../../keys/public.pem"))
            .expect("Error from the decoding key"),
        &Validation::new(jsonwebtoken::Algorithm::RS256), // default validates `exp` automatically
    )
    .map_err(|_| crate::error::ApiError::Gen("invalid or expired token"))?;
    Ok(data.claims)
}
