use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};

use crate::auth::{
    claims::Claims,
    error::{AuthError, AuthResult},
};
#[derive(Debug)]
pub struct JwtVerifier {
    key: DecodingKey,
    validation: Validation,
}

impl JwtVerifier {
    pub fn from_pem(public_pem: &[u8]) -> anyhow::Result<Self> {
        let key = DecodingKey::from_rsa_pem(public_pem)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        Ok(Self { key, validation })
    }

    pub fn verify_token(&self, token: &str) -> AuthResult<Claims> {
        let data = decode::<Claims>(token, &self.key, &self.validation).map_err(|e| {
            tracing::warn!(error = %e, "JWT verification failed");
            AuthError::Internal(e.into())
        })?;

        Ok(data.claims)
    }
}
