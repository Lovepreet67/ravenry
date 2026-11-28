use std::sync::Arc;

use iam_contracts::destination_delivery::DeliveryOutcome;
use tracing::instrument;
use uuid::Uuid;

use crate::application::delivery::{
    http_publisher::HttpPublisher,
    model::DeliveryRequest,
    retry::{RetryDecision, RetryPolicy},
};

#[derive(Clone)]
pub struct DeliveryService {
    publisher: Arc<HttpPublisher>,
}

impl DeliveryService {
    pub fn new(publisher: Arc<HttpPublisher>) -> Self {
        Self { publisher }
    }

    #[instrument(skip_all,fields(delivery_id=%request.delivery_id))]
    pub async fn execute_delivery(
        &self,
        request: DeliveryRequest,
    ) -> anyhow::Result<DeliveryOutcome> {
        let attempt_id = Uuid::new_v4();

        let mut outcome = self
            .publisher
            .deliver(&request, attempt_id, request.curr_attempt.unwrap_or(1))
            .await;
        if outcome.success {
            return Ok(outcome);
        }
        let retry_policy = RetryPolicy::new(
            request.destination_config.config.max_attempts,
            request.destination_config.config.retry_delay_ms,
        );

        match retry_policy.on_failure(request.curr_attempt.unwrap_or(1)) {
            RetryDecision::RetryAt(next_attempt_at) => {
                outcome.will_retry = true;
                outcome.next_attempt_at = Some(next_attempt_at);
            }

            RetryDecision::GiveUp => {
                outcome.will_retry = false;
            }
        }
        Ok(outcome)
    }

    // /// Derives a stable delivery ID from the event and destination IDs.
    pub fn delivery_id(event_id: Uuid, destination_id: Uuid) -> Uuid {
        let name = format!("{event_id}:{destination_id}");

        Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
    }
}
