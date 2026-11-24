use std::time::Duration;

use chrono::Utc;
use iam_contracts::delivery_outcome::DeliveryOutcome;
use reqwest::Client;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::application::delivery::model::DeliveryRequest;

#[derive(Clone)]
pub struct HttpPublisher {
    client: Client,
}

impl HttpPublisher {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()?;

        Ok(Self { client })
    }

    #[instrument(skip_all,fields(delivery_id=%request.delivery_id))]
    pub async fn deliver(
        &self,
        request: &DeliveryRequest,
        attempt_id: Uuid,
        attempt: u32,
    ) -> DeliveryOutcome {
        let attempted_at = Utc::now();

        let destination = &request.destination_config;
        let result = self
            .client
            .post(&request.destination_config.config.url)
            .timeout(Duration::from_secs(destination.config.timeout))
            .header("Idempotency-Key", request.delivery_id.to_string())
            .json(&request.payload)
            .send()
            .await;

        match result {
            Ok(response) => {
                let status = response.status();
                let success = status.is_success();

                DeliveryOutcome {
                    schema_version: 1,
                    delivery_id: request.delivery_id,
                    attempt_id,
                    attempt,
                    event_id: request.event_id,
                    tenant_id: request.tenant_id,
                    topic_id: request.topic_id,
                    destination_id: request.destination_id,
                    success,
                    will_retry: false,
                    status_code: Some(status.as_u16()),
                    error: if success {
                        None
                    } else {
                        Some(format!("destination returned HTTP {status}"))
                    },
                    attempted_at,
                    next_attempt_at: None,
                }
            }

            Err(err) => DeliveryOutcome {
                schema_version: 1,
                delivery_id: request.delivery_id,
                attempt_id,
                attempt,
                event_id: request.event_id,
                tenant_id: request.tenant_id,
                topic_id: request.topic_id,
                destination_id: request.destination_id,
                success: false,
                will_retry: false,
                status_code: err.status().map(|status| status.as_u16()),
                error: Some(err.to_string()),
                attempted_at,
                next_attempt_at: None,
            },
        }
    }
}
