use std::matches;

use axum::{RequestPartsExt, extract::FromRequestParts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    auth::model::Claims,
    domain::membership::model::MembershipDetails,
    shared::{
        auth::jwt::verify_token,
        error::{ApiError, ApiResult},
    },
    state::AppState,
};

pub enum AuthContext {
    Admin,
    User(UserContext),
}

pub struct UserContext {
    pub memberships: Vec<MembershipDetails>,
    pub username: String,
    pub email: String,
    pub user_id: Uuid,
}

impl From<Claims> for AuthContext {
    fn from(value: Claims) -> Self {
        let user_context = UserContext {
            memberships: value.memberships,
            username: value.sub,
            email: value.email,
            user_id: value.user_id,
        };
        return Self::User(user_context);
    }
}

impl AuthContext {
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
    pub fn assert_tenant_access(
        &self,
        tenant_id: uuid::Uuid,
        role_id: Option<uuid::Uuid>,
    ) -> ApiResult<()> {
        match self {
            AuthContext::Admin => Ok(()),
            AuthContext::User(u) => {
                let membership = match u.memberships.iter().find(|m| m.tenant_id == tenant_id) {
                    None => return Err(ApiError::Forbidden),
                    Some(m) => m,
                };
                if let Some(role) = role_id {
                    if membership.role_id != role {
                        return Err(ApiError::Forbidden);
                    }
                }

                Err(ApiError::Forbidden)
            }
        }
    }
}

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
        let claims = verify_token(provided).map_err(|_| ApiError::Unauthorized)?;
        Ok(claims.into())
    }
}
