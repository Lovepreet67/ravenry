use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use iam_contracts::auth::{context::AuthContext, error::AuthError};

use crate::state::AppState;

impl FromRequestParts<AppState> for AuthContext {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AuthError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .filter(|token| !token.is_empty())
            .ok_or(AuthError::Unauthorized)?;

        let auth_context: AuthContext = state.jwt_verifier.verify_token(token)?.into();

        Ok(auth_context)
    }
}
