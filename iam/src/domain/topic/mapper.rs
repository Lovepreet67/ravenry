use crate::domain::topic::{dto, model};

impl model::NewTopic {
    pub fn from_req(req: dto::CreateTopicRequest, tenant_id: uuid::Uuid, created_by: &str) -> Self {
        Self {
            name: req.name,
            description: req.description,
            tenant_id: tenant_id,
            path: req.path,
            created_by: created_by.into(),
        }
    }
}

impl From<model::Topic> for dto::Topic {
    fn from(value: model::Topic) -> Self {
        Self {
            id: value.id,
            name: value.name,
            path: value.path,
            description: value.description,
            tenant_id: value.tenant_id,
            is_active: value.is_active,
            created_by: value.created_by,
            last_updated_by: value.last_updated_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl model::Topic {
    pub fn updated_destination(
        mut existing: model::Topic,
        update_req: dto::UpdateTopicRequest,
    ) -> Self {
        if let Some(name) = update_req.name {
            existing.name = name;
        }
        if let Some(description) = update_req.description {
            existing.description = Some(description);
        }
        if let Some(is_active) = update_req.is_active {
            existing.is_active = is_active;
        }

        existing
    }
}
