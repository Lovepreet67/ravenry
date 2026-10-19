use crate::domain::tenant::{
    dto::{CreateTenantRequest, TenantResponse},
    model::{NewTenant, Tenant},
};

impl From<Tenant> for TenantResponse {
    fn from(value: Tenant) -> Self {
        Self {
            id: value.id,
            name: value.name,
            slug: value.slug,
            is_active: value.is_active,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl NewTenant {
    pub fn from_req(value: CreateTenantRequest, default_role_id: uuid::Uuid) -> Self {
        Self {
            name: value.name,
            slug: value.slug,
            default_role_id: default_role_id,
            is_active: true,
        }
    }
}
