use axum::{RequestPartsExt, extract::FromRequestParts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use subtle::ConstantTimeEq;

use crate::{shared::error::ApiError, state::AppState};
pub type AuthContext = iam_contracts::auth::context::AuthContext;

impl FromRequestParts<AppState> for AuthContext {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // first we will check if the x-admin-api-key header is set
        if let Some(provided_admin_key) = parts.headers.get("X-Admin-API-Key") {
            let provided = provided_admin_key
                .to_str()
                .map_err(|e| ApiError::BadRequest("Invalid Admin Key Provided".into()))?;
            if provided
                .as_bytes()
                .ct_eq(state.config.admin_api_key.as_bytes())
                .into()
            {
                return Ok(AuthContext::Admin);
            }
            return Err(ApiError::BadRequest("Invalid Admin Key Provided".into()));
        }
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| ApiError::BadRequest("missing bearer token".into()))?;
        let provided = bearer.token();

        // first we will verify if user is jwt authenticated
        let claims = state
            .jwt_verifier
            .verify_token(provided)
            .map_err(|_| ApiError::Unauthorized)?;
        Ok(claims.into())
    }
}
