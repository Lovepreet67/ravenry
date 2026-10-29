use std::{collections::HashSet, print, println, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use iam_contracts::{events::PublishedEvent, streams::DELIVERY_RESULTS};
use streamforge::{message::Message, traits::Queue};
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::{
    application::delivery::service::DeliveryService,
    platform::topic_registry::{TopicRegistry, TopicStream},
};

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub consumer_group: String,
    pub consumer_name: String,
    pub batch_size: usize,
    pub registry_refresh_interval: Duration,
    pub idle_interval: Duration,
}

pub struct OutboundWorker {
    queue: Arc<dyn Queue>,
    registry: Arc<dyn TopicRegistry>,
    delivery: DeliveryService,
    config: WorkerConfig,
    initialized_streams: HashSet<String>,
}

impl OutboundWorker {
    pub fn new(
        queue: Arc<dyn Queue>,
        registry: Arc<dyn TopicRegistry>,
        delivery: DeliveryService,
        config: WorkerConfig,
    ) -> Self {
        Self {
            queue,
            registry,
            delivery,
            config,
            initialized_streams: HashSet::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        println!("Inside run");
        let mut next_refresh = tokio::time::Instant::now();

        loop {
            if tokio::time::Instant::now() >= next_refresh {
                next_refresh = tokio::time::Instant::now() + self.config.registry_refresh_interval;

                match self.registry.list_active().await {
                    Ok(streams) => {
                        for stream in streams {
                            if let Err(err) = self.process_stream(&stream).await {
                                error!(
                                    tenant_id = %stream.tenant_id,
                                    topic_id = %stream.topic_id,
                                    error = %err,
                                    "failed to process topic stream"
                                );
                            }
                        }
                    }
                    Err(err) => {
                        error!(
                            error = %err,
                            "failed to refresh active-topic registry"
                        );
                    }
                }
            }

            sleep(self.config.idle_interval).await;
        }
    }

    async fn process_stream(&mut self, stream: &TopicStream) -> Result<()> {
        info!("Processing stream {:?}", stream);
        let destination = stream.destination();

        if self.initialized_streams.insert(destination.clone()) {
            self.queue
                .ensure_group(&destination, &self.config.consumer_group)
                .await
                .with_context(|| format!("failed to create consumer group for {destination}"))?;
        }

        let deliveries = self
            .queue
            .consume(
                &destination,
                &self.config.consumer_group,
                &self.config.consumer_name,
                self.config.batch_size,
                0,
            )
            .await
            .with_context(|| format!("failed to consume from {destination}"))?;

        println!("delivery : {:?}", deliveries);
        for delivery in deliveries {
            let event: PublishedEvent = match serde_json::from_slice(&delivery.message.payload) {
                Ok(event) => event,
                Err(err) => {
                    // Do not silently discard malformed messages.
                    // Leave this entry pending for later recovery or
                    // explicit dead-letter handling.
                    warn!(
                        stream = %destination,
                        delivery_id = %delivery.delivery_id,
                        error = %err,
                        "invalid published event"
                    );
                    continue;
                }
            };

            if event.tenant_id != stream.tenant_id || event.topic_id != stream.topic_id {
                warn!(
                    stream = %destination,
                    delivery_id = %delivery.delivery_id,
                    "event tenant/topic does not match its stream"
                );
                continue;
            }

            let outcomes = match self.delivery.deliver_event(&event).await {
                Ok(outcomes) => outcomes,
                Err(err) => {
                    warn!(
                        event_id = %event.event_id,
                        error = %err,
                        "event delivery processing failed"
                    );
                    continue;
                }
            };

            // Publish each outcome before acknowledging the source event.
            let mut results_published = true;

            for outcome in outcomes {
                let payload = match serde_json::to_vec(&outcome) {
                    Ok(payload) => payload,
                    Err(err) => {
                        error!(
                            event_id = %event.event_id,
                            error = %err,
                            "failed to serialize delivery outcome"
                        );
                        results_published = false;
                        break;
                    }
                };

                if let Err(err) = self
                    .queue
                    .publish(DELIVERY_RESULTS, Message::new(payload))
                    .await
                {
                    error!(
                        event_id = %event.event_id,
                        error = %err,
                        "failed to publish delivery outcome"
                    );
                    results_published = false;
                    break;
                }
            }

            if !results_published {
                // Leave the source entry pending rather than lose outcomes.
                continue;
            }

            self.queue
                .ack(
                    &destination,
                    &self.config.consumer_group,
                    &delivery.delivery_id,
                )
                .await
                .with_context(|| {
                    format!("failed to acknowledge delivery {}", delivery.delivery_id)
                })?;

            info!(
                event_id = %event.event_id,
                delivery_id = %delivery.delivery_id,
                "published event processed"
            );
        }

        Ok(())
    }
}
