use std::sync::Arc;

use metrics::counter;
use mimir::names;
use streamforge::{message::Message, traits::Queue};
use uuid::Uuid;

use crate::{
    application::retry::{command::RetryCommand, error::RetryError},
    platform::projection_store::ProjectionStore,
};

pub struct RetryService {
    queue: Arc<dyn Queue>,
    projections: ProjectionStore,
}

impl RetryService {
    pub fn new(queue: Arc<dyn Queue>, projections: ProjectionStore) -> Self {
        Self { queue, projections }
    }

    pub async fn initiate_retry(
        &self,
        tenant_id: Uuid,
        command: RetryCommand,
    ) -> Result<String, RetryError> {
        counter!(names::MANUAL_RETRY_RECEIVED).increment(1);
        let attempts = self
            .projections
            .assert_delivery_exist(tenant_id, command.destination_id, command.delivery_id)
            .await
            .map_err(|err| RetryError::InvalidRequest(err.to_string()))?;
        let event = iam_contracts::events::RetryEvent::new(
            command.delivery_id,
            tenant_id,
            command.destination_id,
            attempts,
        );

        let payload = serde_json::to_vec(&event)
            .map_err(|err| RetryError::InvalidRequest(err.to_string()))?;

        let message = Message::new(payload).with_id(event.delivery_id.to_string());
        let destination = iam_contracts::streams::MANUALY_RETRY;

        let message_id = self
            .queue
            .publish(&destination, message)
            .await
            .map_err(|_| RetryError::QueueUnavailable)?;
        self.projections
            .mark_delivery_pending(tenant_id, command.destination_id, command.delivery_id)
            .await
            .map_err(|e| RetryError::InvalidRequest(e.to_string()))?;
        counter!(names::MANUAL_RETRY_PUBLISHED).increment(1);
        Ok(message_id)
    }
}
