use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

use crate::auth::{
    claims::Claims,
    error::{AuthError, AuthResult},
};

#[derive(Debug)]
pub struct JwtSigner {
    key: EncodingKey,
}

impl JwtSigner {
    pub fn from_pem(private_pem: &[u8]) -> anyhow::Result<Self> {
        let key = EncodingKey::from_rsa_pem(private_pem)?;
        Ok(Self { key })
    }

    pub fn sign(&self, claims: &Claims) -> AuthResult<String> {
        encode(&Header::new(Algorithm::RS256), claims, &self.key).map_err(|e| {
            tracing::error!(error = %e, "Failed to sign JWT");
            AuthError::Internal(e.into())
        })
    }
}
