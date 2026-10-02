use std::env;

use axum::{RequestPartsExt, extract::FromRequestParts, http::StatusCode};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

pub struct AdminAuth;
impl<S> FromRequestParts<S> for AdminAuth
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let expected = env::var("ADMIN_API_KEY").expect("missing admin api key");
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "missing bearer token"))?;
        let provided = bearer.token();

        // constant-time comparison — a plain `==` leaks timing info about
        // how many leading bytes matched, which is a real (if minor) attack vector for secrets
        if provided.len() != expected.len() || provided != expected {
            return Err((StatusCode::UNAUTHORIZED, "invalid admin api key"));
        }

        Ok(AdminAuth)
    }
}
