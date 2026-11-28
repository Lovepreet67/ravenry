use std::time::Duration;

use chrono::Utc;
use iam_contracts::destination_delivery::DeliveryOutcome;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::instrument;
use uuid::Uuid;

use crate::application::delivery::model::DeliveryRequest;

const MAX_BODY_BYTES: usize = 16 * 1024;

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
    /// Reads at most MAX_BODY_BYTES of the body. Parsed as JSON if complete
    /// and valid, otherwise stored as (lossy) text.
    async fn capture_body(mut response: reqwest::Response) -> Value {
        let mut buf: Vec<u8> = Vec::new();
        let mut truncated = false;

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    let room = MAX_BODY_BYTES - buf.len();
                    if chunk.len() > room {
                        buf.extend_from_slice(&chunk[..room]);
                        truncated = true;
                        break;
                    }
                    buf.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => {
                    return json!({ "read_error": e.to_string(), "truncated": true });
                }
            }
        }

        let body = if truncated {
            Value::String(String::from_utf8_lossy(&buf).into_owned())
        } else {
            serde_json::from_slice(&buf)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&buf).into_owned()))
        };
        json!({ "body": body, "truncated": truncated })
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
                let raw_response = Some(Self::capture_body(response).await);

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
                    raw_response,
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
                raw_response: Some(json!({
                    "transport_error": { "timeout": err.is_timeout(), "connect": err.is_connect() }
                })),
                status_code: err.status().map(|status| status.as_u16()),
                error: Some(err.to_string()),
                attempted_at,
                next_attempt_at: None,
            },
        }
    }
}
