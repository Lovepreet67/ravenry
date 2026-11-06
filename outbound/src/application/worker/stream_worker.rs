use std::{format, sync::Arc, time::Duration};

use streamforge::traits::Queue;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    application::{delivery::service::DeliveryService, worker::util::sleep_or_cancel},
    config::worker::StreamWorkerConfig,
};

pub struct StreamWorker {
    id: Uuid,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl StreamWorker {
    pub fn run(
        id: Uuid,
        stream: String,
        queue: Arc<dyn Queue>,
        delivery_service: Arc<DeliveryService>,
        config: Arc<StreamWorkerConfig>,
        token: CancellationToken,
    ) -> Self {
        let token_clone = token.clone();
        let handle = tokio::spawn(async move {
            // Each consumer in a Redis consumer group needs a unique name.
            let consumer_name = format!("{}_{}", config.consumer_name_prefix, id);
            let idle_interval = Duration::from_secs(config.idle_interval_secs);

            while !token.is_cancelled() {
                let consume = queue.consume(
                    &stream,
                    &config.consumer_group,
                    &consumer_name,
                    config.batch_size,
                    1000,
                );

                let deliveries = tokio::select! {
                    _ = token.cancelled() => break,
                    result = consume => {
                        match result {
                            Ok(deliveries) => deliveries,
                            Err(err) => {
                                // warn!(
                                //     stream = %stream,
                                //     consumer = %consumer_name,
                                //     error = %err,
                                //     "Consume failed"
                                // );

                                sleep_or_cancel(
                                    &token,
                                    Duration::from_secs(1),
                                ).await;

                                continue;
                            }
                        }
                    }
                };

                if deliveries.is_empty() {
                    sleep_or_cancel(&token, idle_interval).await;

                    continue;
                }

                for delivery in deliveries {
                    if token.is_cancelled() {
                        break;
                    }

                    let event: iam_contracts::events::PublishedEvent =
                        match serde_json::from_slice(&delivery.message.payload) {
                            Ok(event) => event,
                            Err(err) => {
                                warn!(
                                    stream = %stream,
                                    queue_message_id = %delivery.delivery_id,
                                    error = %err,
                                    "Invalid published event"
                                );

                                // Do not ACK malformed messages.
                                // Handle them through source-message DLQ
                                // or an explicit poison-message policy.
                                continue;
                            }
                        };

                    if let Err(err) = delivery_service.deliver_event(&event).await {
                        warn!(
                            event_id = %event.event_id,
                            error = %err,
                            "Event processing failed"
                        );

                        // Leave the source message pending.
                        continue;
                    }

                    if let Err(err) = queue
                        .ack(&stream, &config.consumer_group, &delivery.delivery_id)
                        .await
                    {
                        warn!(
                            stream = %stream,
                            queue_message_id = %delivery.delivery_id,
                            error = %err,
                            "Source event ACK failed"
                        );

                        continue;
                    }

                    info!(
                        stream = %stream,
                        consumer = %consumer_name,
                        event_id = %event.event_id,
                        queue_message_id = %delivery.delivery_id,
                        "Event processing completed"
                    );
                }
            }

            info!(
                stream = %stream,
                consumer = %consumer_name,
                "Outbound worker stopped"
            );
        });
        Self {
            id,
            token: token_clone,
            handle,
        }
    }
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
    pub async fn cancel(self) {
        self.token.cancel();
        if let Err(err) = self.handle.await {
            warn!(
                stream_worker = %self.id,
                error = %err,
                "Stream worker join failed"
            );
        }
    }
}
