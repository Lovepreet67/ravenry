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

        let event = iam_contracts::events::PublishedEvent::new(
            tenant_id,
            topic_id,
            command.topic_path,
            command.payload,
        );

        let payload = serde_json::to_vec(&event)
            .map_err(|err| PublishError::InvalidRequest(err.to_string()))?;

        let message = Message::new(payload).with_id(event.event_id.to_string());
        let destination = iam_contracts::streams::topic_events(tenant_id, topic_id);

        self.queue
            .publish(&destination, message)
            .await
            .map_err(|_| PublishError::QueueUnavailable)
    }
}
