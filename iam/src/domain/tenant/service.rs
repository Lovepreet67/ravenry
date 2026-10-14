use std::sync::Arc;

use crate::{
    domain::tenant::{
        dto::{CreateTenantRequest, TenantFilter, TenantResponse, UpdateTenantRequest},
        model::NewTenant,
        repository::TenantRepository,
    },
    shared::{
        error::{ApiError, ApiResult},
        pagination::{PaginatedList, Pagination},
    },
};

pub async fn create_tenant(
    repo: &Arc<dyn TenantRepository>,
    req: CreateTenantRequest,
) -> ApiResult<TenantResponse> {
    let new_tenant: NewTenant = req.into();
    let tenant = repo.create(&new_tenant).await?;
    Ok(tenant.into())
}

pub async fn get_tenant(repo: &Arc<dyn TenantRepository>, id: &str) -> ApiResult<TenantResponse> {
    let tenant = repo.get(id).await?.ok_or(ApiError::NotFound)?;
    Ok(tenant.into())
}

pub async fn get_tenant_detail(
    repo: &Arc<dyn TenantRepository>,
    id: &str,
) -> ApiResult<TenantResponse> {
    let tenant = repo.get(id).await?.ok_or(ApiError::NotFound)?;
    Ok(tenant.into())
}

pub async fn list_tenants(
    repo: &Arc<dyn TenantRepository>,
    pagination: &Pagination,
    filter: &TenantFilter,
) -> ApiResult<PaginatedList<TenantResponse>> {
    let result = repo.list(filter, pagination).await?;
    Ok(PaginatedList {
        list: result.list.into_iter().map(Into::into).collect(),
        page_info: result.page_info,
    })
}

pub async fn update_tenant(
    repo: &Arc<dyn TenantRepository>,
    id: &str,
    req: UpdateTenantRequest,
) -> ApiResult<TenantResponse> {
    // load-modify-save: fetch current state, apply only the fields the client sent
    let mut tenant = repo.get(id).await?.ok_or(ApiError::NotFound)?;

    if let Some(name) = req.name {
        tenant.name = name;
    }
    if let Some(is_active) = req.is_active {
        tenant.is_active = is_active;
    }

    let updated = repo.update(&tenant).await?;
    Ok(updated.into())
}
