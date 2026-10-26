use std::{collections::HashSet, sync::Arc};

use streamforge::{message::Message, traits::Queue};
use uuid::Uuid;

use crate::{
    application::publish::{command::PublishCommand, error::PublishError},
    platform::projection_store::ProjectionStore,
};

pub struct PublishService {
    queue: Arc<dyn Queue>,
    projections: ProjectionStore,
    publish_roles: Arc<HashSet<String>>,
}

impl PublishService {
    pub fn new(
        queue: Arc<dyn Queue>,
        projections: ProjectionStore,
        publish_roles: Arc<HashSet<String>>,
    ) -> Self {
        Self {
            queue,
            projections,
            publish_roles,
        }
    }

    pub async fn publish(
        &self,
        tenant_id: Uuid,
        command: PublishCommand,
    ) -> Result<String, PublishError> {
        let topic_id = self
            .projections
            .resolve_topic_id(tenant_id, &command.topic_path)
            .await
            .map_err(|_| PublishError::ProjectionUnavailable)?
            .ok_or(PublishError::TopicNotFound)?;

        let payload = serde_json::to_vec(&command.payload)
            .map_err(|err| PublishError::InvalidRequest(err.to_string()))?;

        let message = Message::new(payload);

        self.queue
            .publish(&topic_id.to_string(), message)
            .await
            .map_err(|_| PublishError::QueueUnavailable)
    }
}
