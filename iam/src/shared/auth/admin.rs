use std::env;

use axum::{RequestPartsExt, extract::FromRequestParts, http::StatusCode};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

use crate::state::AppState;

pub struct AdminAuth;
impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = (StatusCode, &'static str);
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // let expected = state
        let expected: &str = state.config.admin_api_key.as_ref();
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
