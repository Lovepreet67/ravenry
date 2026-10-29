use std::{println, time::Duration};

use reqwest::{Client, StatusCode};
use serde::Serialize;
use uuid::Uuid;

use crate::model::ResolvedDestination;

#[derive(Debug, Clone, Serialize)]
pub struct DeliveryOutcome {
    pub event_id: Uuid,
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
    pub destination_id: Uuid,
    pub success: bool,
    pub status_code: Option<u16>,
    pub error: Option<String>,
}

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

    pub async fn deliver(
        &self,
        destination: &ResolvedDestination,
        event_id: Uuid,
        tenant_id: Uuid,
        topic_id: Uuid,
        payload: &serde_json::Value,
    ) -> DeliveryOutcome {
        let result = self
            .client
            .post(&destination.config.url)
            .timeout(Duration::from_secs(destination.config.timeout))
            .json(payload)
            .send()
            .await;
        println!("{:?}", result);

        match result {
            Ok(response) => {
                let status = response.status();

                DeliveryOutcome {
                    event_id,
                    tenant_id,
                    topic_id,
                    destination_id: destination.id,
                    success: status.is_success(),
                    status_code: Some(status.as_u16()),
                    error: if status.is_success() {
                        None
                    } else {
                        Some(format!("destination returned HTTP {status}"))
                    },
                }
            }
            Err(err) => DeliveryOutcome {
                event_id,
                tenant_id,
                topic_id,
                destination_id: destination.id,
                success: false,
                status_code: err.status().map(|status: StatusCode| status.as_u16()),
                error: Some(err.to_string()),
            },
        }
    }
}
