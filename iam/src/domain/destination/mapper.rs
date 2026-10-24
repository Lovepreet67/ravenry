use crate::domain::destination::{dto, model};

impl model::NewDestination {
    pub fn from_req(
        req: dto::CreateDestinationRequest,
        tenant_id: uuid::Uuid,
        created_by: &str,
    ) -> Self {
        Self {
            name: req.name,
            kind: req.kind,
            description: req.description,
            tenant_id: tenant_id,
            destination_url: req.destination_url,
            delivery_timeout_seconds: req.delivery_timeout_seconds,
            created_by: created_by.into(),
        }
    }
}

impl From<model::Destination> for dto::Destination {
    fn from(value: model::Destination) -> Self {
        Self {
            id: value.id,
            name: value.name,
            kind: value.kind,
            destination_url: value.destination_url,
            is_active: value.is_active,
            delivery_timeout_seconds: value.delivery_timeout_seconds,
            created_by: value.created_by,
            last_updated_by: value.last_updated_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl model::Destination {
    pub fn updated_destination(
        mut existing: model::Destination,
        update_req: dto::UpdateDestinationRequest,
    ) -> Self {
        if let Some(name) = update_req.name {
            existing.name = name;
        }
        if let Some(description) = update_req.description {
            existing.description = Some(description);
        }

        if let Some(destination_url) = update_req.destination_url {
            existing.destination_url = destination_url;
        }
        if let Some(delivery_timeout_seconds) = update_req.delivery_timeout_seconds {
            existing.delivery_timeout_seconds = delivery_timeout_seconds;
        }
        if let Some(kind) = update_req.kind {
            existing.kind = kind;
        }
        if let Some(is_active) = update_req.is_active {
            existing.is_active = is_active;
        }

        existing
    }
}
