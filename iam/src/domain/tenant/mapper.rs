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

impl From<CreateTenantRequest> for NewTenant {
    fn from(value: CreateTenantRequest) -> Self {
        Self {
            name: value.name,
            slug: value.slug,
            is_active: true,
        }
    }
}
