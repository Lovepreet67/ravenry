use std::matches;

use uuid::Uuid;

use crate::auth::{
    claims::Claims,
    error::{AuthError, AuthResult},
    model::MembershipDetails,
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
    pub fn assert_is_admin(&self) -> AuthResult<()> {
        if !matches!(self, Self::Admin) {
            return Err(AuthError::Unauthorized);
        }
        Ok(())
    }
    pub fn assert_tenant_access(
        &self,
        tenant_id: uuid::Uuid,
        role_slug: Option<&str>,
    ) -> AuthResult<()> {
        match self {
            AuthContext::Admin => Ok(()),
            AuthContext::User(u) => {
                let memberships = u
                    .memberships
                    .iter()
                    .filter(|m| m.tenant_id == tenant_id)
                    .collect::<Vec<_>>();
                if memberships.len() == 0 {
                    return Err(AuthError::Forbidden);
                }
                if let Some(role) = role_slug {
                    if memberships.iter().find(|m| m.role_slug == role).is_none() {
                        return Err(AuthError::Forbidden);
                    }
                }

                Ok(())
            }
        }
    }
    pub fn assert_tenant_any_access(
        &self,
        tenant_id: uuid::Uuid,
        role_slugs: &[String],
    ) -> AuthResult<()> {
        match self {
            AuthContext::Admin => Ok(()),

            AuthContext::User(u) => {
                let has_access = u
                    .memberships
                    .iter()
                    .any(|m| m.tenant_id == tenant_id && role_slugs.contains(&m.role_slug));

                if has_access {
                    Ok(())
                } else {
                    Err(AuthError::Forbidden)
                }
            }
        }
    }
    pub fn assert_user_access(
        &self,
        user_id: uuid::Uuid,
        tenant_id: Option<uuid::Uuid>,
    ) -> AuthResult<()> {
        match self {
            AuthContext::Admin => Ok(()),
            AuthContext::User(u) => {
                if u.user_id == user_id {
                    return Ok(());
                }
                // user can access the specified user_id if logged in user is admin for specified tenant
                if let Some(tenant_id) = tenant_id
                    && u.memberships
                        .iter()
                        .find(|m| m.role_slug == "ownwer" && m.tenant_id == tenant_id)
                        .is_some()
                {
                    return Ok(());
                }
                return Err(AuthError::Unauthorized);
            }
        }
    }
    pub fn get_email(&self) -> String {
        match self {
            AuthContext::Admin => "admin".into(),
            AuthContext::User(u) => return u.email.clone(),
        }
    }
}
