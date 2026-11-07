use iam_contracts::auth::{context::AuthContext, error::AuthError};
use uuid::Uuid;

/// Proof that the caller is allowed to read this tenant's data.
///
/// Fields are private and the only constructor is `authorize`, so every
/// repository call that takes a `&TenantScope` is guaranteed to have been
/// authorized first.
#[derive(Debug, Clone, Copy)]
pub struct TenantScope {
    tenant_id: Uuid,
    can_view_payload: bool,
}

impl TenantScope {
    /// Any membership in the tenant grants read access. Roles listed in
    /// `payload_roles` (from config) may also see event payloads, which
    /// can contain sensitive customer data.
    pub fn authorize(
        auth: &AuthContext,
        tenant_id: Uuid,
        payload_roles: &[String],
    ) -> Result<Self, AuthError> {
        auth.assert_tenant_access(tenant_id, None)?;

        let can_view_payload = match auth {
            AuthContext::Admin => true,
            AuthContext::User(u) => u
                .memberships
                .iter()
                .any(|m| m.tenant_id == tenant_id && payload_roles.contains(&m.role_slug)),
        };

        Ok(Self { tenant_id, can_view_payload })
    }

    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    pub fn can_view_payload(&self) -> bool {
        self.can_view_payload
    }
}