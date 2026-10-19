use jsonwebtoken::{
    Algorithm::RS256, DecodingKey, EncodingKey, Header, Validation, decode, encode,
};

use crate::{auth::model::Claims, shared::error::ApiResult};

pub fn sign(claims: Claims) -> ApiResult<String> {
    let token = encode(
        &Header::new(RS256), // default = HS256
        &claims,
        &EncodingKey::from_rsa_pem(include_bytes!("../../../keys/private.pem"))
            .expect("Error from rsa key"),
    )
    .map_err(|e| {
        tracing::error!("{:?}", e);
        return crate::shared::error::ApiError::Internal(e.into());
    })?;

    Ok(token)
}

pub fn verify_token(token: &str) -> ApiResult<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_rsa_pem(include_bytes!("../../../keys/public.pem"))
            .expect("Error from the decoding key"),
        &Validation::new(jsonwebtoken::Algorithm::RS256), // default validates `exp` automatically
    )
    .map_err(|e| crate::shared::error::ApiError::Internal(e.into()))?;
    Ok(data.claims)
}
