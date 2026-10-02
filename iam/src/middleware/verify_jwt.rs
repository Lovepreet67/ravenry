use axum::{
    RequestPartsExt,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::Serialize;
use uuid::Uuid;

use crate::{model::user::UserMembership, service::auth::verify_token, state::AppState};

#[derive(Debug, Serialize)]
pub struct CurrentUser {
    pub user_id: Uuid,
    pub username: String,
    pub memberships: Vec<UserMembership>,
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>, // lets us pull AppState out of whatever state type S is
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "missing or invalid authorization header",
                )
            })?;

        let claims = verify_token(bearer.token())
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid or expired token"))?;

        Ok(CurrentUser {
            user_id: claims.sub,
            username: claims.username,
            memberships: claims.memberships,
        })
    }
}
