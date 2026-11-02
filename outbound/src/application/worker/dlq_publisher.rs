use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use iam_contracts::streams;
use streamforge::{traits::Queue, Message};
use tracing::{error, info, warn};

use crate::platform::redis::delivery_store::DeliveryStore;

#[derive(Debug, Default)]
pub struct DlqPublisherStats {
    pub destinations_scanned: usize,
    pub messages_found: usize,
    pub messages_published: usize,
    pub failures: usize,
}

pub struct DlqPublisher {
    store: DeliveryStore,
    queue: Arc<dyn Queue>,
    batch_size: usize,
}

impl DlqPublisher {
    pub fn new(store: DeliveryStore, queue: Arc<dyn Queue>, batch_size: usize) -> Self {
        Self {
            store,
            queue,
            batch_size: batch_size.max(1),
        }
    }

    pub async fn run_once(&self) -> Result<DlqPublisherStats> {
        let destinations = self
            .store
            .list_pending_dlq_destinations()
            .await
            .context("failed to list pending DLQ destinations")?;

        let mut stats = DlqPublisherStats {
            destinations_scanned: destinations.len(),
            ..Default::default()
        };

        for destination in destinations {
            let messages = match self
                .store
                .get_pending_dlq_messages(
                    destination.tenant_id,
                    destination.destination_id,
                    self.batch_size,
                )
                .await
            {
                Ok(messages) => messages,
                Err(error) => {
                    stats.failures += 1;
                    error!(
                        tenant_id = %destination.tenant_id,
                        destination_id = %destination.destination_id,
                        error = %error,
                        "Failed to load pending DLQ messages"
                    );
                    continue;
                }
            };

            stats.messages_found += messages.len();

            let queue_destination =
                streams::destination_dlq(destination.tenant_id, destination.destination_id);

            for (delivery_id, dlq_message) in messages {
                let payload = match serde_json::to_vec(&dlq_message) {
                    Ok(payload) => payload,
                    Err(error) => {
                        stats.failures += 1;
                        error!(
                            delivery_id = %delivery_id,
                            error = %error,
                            "Failed to serialize DLQ message"
                        );
                        continue;
                    }
                };

                let message = Message::new(payload)
                    .with_id(delivery_id.to_string())
                    .with_header("message_type", "delivery_dlq")
                    .with_header("schema_version", "1")
                    .with_header("delivery_id", delivery_id.to_string())
                    .with_header("tenant_id", destination.tenant_id.to_string())
                    .with_header("destination_id", destination.destination_id.to_string());

                match self.queue.publish(&queue_destination, message).await {
                    Ok(_) => {
                        if let Err(error) = self
                            .store
                            .acknowledge_dlq_message(
                                destination.tenant_id,
                                destination.destination_id,
                                delivery_id,
                            )
                            .await
                        {
                            stats.failures += 1;
                            error!(
                                delivery_id = %delivery_id,
                                error = %error,
                                "DLQ published but Redis acknowledgement failed"
                            );
                            continue;
                        }

                        stats.messages_published += 1;
                    }

                    Err(error) => {
                        stats.failures += 1;
                        warn!(
                            delivery_id = %delivery_id,
                            error = %error,
                            "DLQ publication failed; message remains pending"
                        );
                    }
                }
            }
        }

        info!(
            destinations_scanned = stats.destinations_scanned,
            messages_found = stats.messages_found,
            messages_published = stats.messages_published,
            failures = stats.failures,
            "DLQ publisher iteration completed"
        );

        Ok(stats)
    }

    pub async fn run(&self, poll_interval: Duration) {
        let mut interval = tokio::time::interval(poll_interval.max(Duration::from_millis(1)));

        loop {
            interval.tick().await;

            if let Err(error) = self.run_once().await {
                error!(
                    error = %error,
                    "DLQ publisher iteration failed"
                );
            }
        }
    }
}
