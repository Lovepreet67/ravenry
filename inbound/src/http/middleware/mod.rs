use anyhow::anyhow;
use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use uuid::Uuid;

use crate::{model::auth::Claims, AppState};

// model/claims.rs

pub fn verify_token(token: &str, key: &[u8]) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_rsa_pem(key).expect("Error from the decoding key"),
        &Validation::new(jsonwebtoken::Algorithm::RS256), // default validates `exp` automatically
    )
    .map_err(|_| anyhow!("invalid or expired token"))?;
    Ok(data.claims)
}

pub struct AuthenticatedTenant {
    pub tenant_id: Uuid,
}

impl<S> FromRequestParts<S> for AuthenticatedTenant
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "missing bearer token"))?;

        let Path(target_tenant_id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(|err| (StatusCode::BAD_REQUEST, "Inavid tenant id"))?;

        // in-process verification — no network call, fast, no new failure mode
        let claims = verify_token(bearer.token(), include_bytes!("../../../keys/public.pem"))
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
        let role = claims
            .memberships
            .iter()
            .find(|memberhship| memberhship.tenant_id == target_tenant_id)
            .map(|memberhip| memberhip.role_id);
        if role.is_none() {
            return Err((StatusCode::UNAUTHORIZED, "missing bearer token"));
        }

        Ok(AuthenticatedTenant {
            tenant_id: target_tenant_id,
        })
    }
}
