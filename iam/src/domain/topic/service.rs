use std::sync::Arc;

use uuid::Uuid;

use crate::{
    domain::topic::{
        dto::{CreateTopicRequest, Topic as TopicDto, TopicFilter, UpdateTopicRequest},
        model::{DestinationLink, NewTopic, Topic},
        repository::TopicRepository,
    },
    shared::{
        error::{ApiError, ApiResult},
        pagination::{PaginatedList, Pagination},
    },
};
pub async fn create(
    topic_repo: &Arc<dyn TopicRepository>,
    tenant_id: Uuid,
    user_id: &str,
    req: CreateTopicRequest,
) -> ApiResult<TopicDto> {
    let new_destination = NewTopic::from_req(req, tenant_id, user_id);
    let created = topic_repo.create(new_destination).await?;
    Ok(created.into())
}

pub async fn get(
    topic_repo: &Arc<dyn TopicRepository>,
    tenant_id: Uuid,
    id: Uuid,
) -> ApiResult<TopicDto> {
    let destination = topic_repo
        .get_by_id(id, tenant_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(destination.into())
}

pub async fn list(
    topic_repo: &Arc<dyn TopicRepository>,
    tenant_id: Uuid,
    filter: &TopicFilter,
    pagination: &Pagination,
) -> ApiResult<PaginatedList<TopicDto>> {
    let result = topic_repo.list(tenant_id, filter, pagination).await?;
    Ok(PaginatedList {
        list: result.list.into_iter().map(Into::into).collect(),
        page_info: result.page_info,
    })
}

pub async fn update(
    topic_repo: &Arc<dyn TopicRepository>,
    tenant_id: Uuid,
    user_id: &str,
    id: Uuid,
    req: UpdateTopicRequest,
) -> ApiResult<TopicDto> {
    let existing = topic_repo
        .get_by_id(id, tenant_id)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    let mut updated = Topic::updated_destination(existing, req);
    updated.last_updated_by = user_id.to_string();

    let saved = topic_repo.update(updated).await?;
    Ok(saved.into())
}

pub async fn put_destination(
    topic_repo: &Arc<dyn TopicRepository>,
    topic_id: Uuid,
    tenant_id: Uuid,
    destination_id: Uuid,
    user_id: &str,
) -> ApiResult<TopicDto> {
    let destination_link = DestinationLink {
        topic_id,
        destination_id,
        tenant_id,
        created_by: user_id.into(),
    };
    let topic = topic_repo.put_destination(destination_link).await?;
    Ok(topic.into())
}
pub async fn delete_destination(
    topic_repo: &Arc<dyn TopicRepository>,
    topic_id: Uuid,
    tenant_id: Uuid,
    destination_id: Uuid,
    user_id: &str,
) -> ApiResult<TopicDto> {
    let destination_link = DestinationLink {
        topic_id,
        destination_id,
        tenant_id,
        created_by: user_id.into(),
    };
    let topic = topic_repo.delete_destination(destination_link).await?;
    Ok(topic.into())
}
