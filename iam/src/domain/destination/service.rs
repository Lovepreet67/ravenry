use std::sync::Arc;

use uuid::Uuid;

use crate::{
    domain::destination::{
        dto::{
            CreateDestinationRequest, Destination as DestinationDto, DestinationFilter,
            UpdateDestinationRequest,
        },
        model::{Destination, NewDestination},
        repository::DestinationRepository,
    },
    shared::{
        error::{ApiError, ApiResult},
        pagination::{PaginatedList, Pagination},
    },
};
pub async fn create(
    destination_repo: &Arc<dyn DestinationRepository>,
    tenant_id: Uuid,
    user_id: &str,
    req: CreateDestinationRequest,
) -> ApiResult<DestinationDto> {
    let new_destination = NewDestination::from_req(req, tenant_id, user_id);
    let created = destination_repo.create(new_destination).await?;
    Ok(created.into())
}

pub async fn get(
    destination_repo: &Arc<dyn DestinationRepository>,
    tenant_id: Uuid,
    id: Uuid,
) -> ApiResult<DestinationDto> {
    let destination = destination_repo
        .get_by_id(id, tenant_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(destination.into())
}

pub async fn list(
    destination_repo: &Arc<dyn DestinationRepository>,
    tenant_id: Uuid,
    filter: &DestinationFilter,
    pagination: &Pagination,
) -> ApiResult<PaginatedList<DestinationDto>> {
    let result = destination_repo.list(tenant_id, filter, pagination).await?;
    Ok(PaginatedList {
        list: result.list.into_iter().map(Into::into).collect(),
        page_info: result.page_info,
    })
}

pub async fn update(
    destination_repo: &Arc<dyn DestinationRepository>,

    tenant_id: Uuid,
    user_id: &str,
    id: Uuid,
    req: UpdateDestinationRequest,
) -> ApiResult<DestinationDto> {
    let existing = destination_repo
        .get_by_id(id, tenant_id)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    let mut updated = Destination::updated_destination(existing, req);
    updated.last_updated_by = user_id.to_string();

    let saved = destination_repo.update(updated).await?;
    Ok(saved.into())
}
