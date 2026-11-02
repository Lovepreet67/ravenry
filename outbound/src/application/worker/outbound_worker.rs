use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use iam_contracts::topic_registory::TopicRegistry;
use streamforge::traits::Queue;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::application::delivery::service::DeliveryService;

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub consumer_group: String,
    pub consumer_name: String,
    pub workers_per_stream: usize,
    pub batch_size: usize,
    pub registry_refresh_interval: Duration,
    pub idle_interval: Duration,
}

struct StreamWorker {
    token: CancellationToken,
    handle: JoinHandle<()>,
}

async fn sleep_or_cancel(token: &CancellationToken, duration: Duration) {
    tokio::select! {
        _ = token.cancelled() => {}
        _ = tokio::time::sleep(duration) => {}
    }
}

pub struct OutboundWorkerSupervisor {
    queue: Arc<dyn Queue>,
    registry: Arc<dyn TopicRegistry>,
    delivery: Arc<DeliveryService>,
    config: WorkerConfig,
}

impl OutboundWorkerSupervisor {
    pub fn new(
        queue: Arc<dyn Queue>,
        registry: Arc<dyn TopicRegistry>,
        delivery: Arc<DeliveryService>,
        config: WorkerConfig,
    ) -> Self {
        Self {
            queue,
            registry,
            delivery,
            config,
        }
    }

    pub async fn run(self: Arc<Self>, shutdown: CancellationToken) {
        // Key: (stream name, worker index)
        let mut workers: HashMap<(String, usize), StreamWorker> = HashMap::new();

        while !shutdown.is_cancelled() {
            if let Err(err) = self.reconcile(&mut workers, &shutdown).await {
                warn!(error = %err, "Outbound reconciliation failed");
            }

            sleep_or_cancel(&shutdown, self.config.registry_refresh_interval).await;
        }

        // Graceful shutdown.
        for (_, worker) in workers.drain() {
            worker.token.cancel();

            if let Err(err) = worker.handle.await {
                warn!(error = %err, "Outbound worker join failed");
            }
        }

        info!("Outbound supervisor stopped");
    }

    async fn reconcile(
        self: &Arc<Self>,
        workers: &mut HashMap<(String, usize), StreamWorker>,
        shutdown: &CancellationToken,
    ) -> Result<()> {
        let streams: HashSet<String> = self
            .registry
            .list_active()
            .await?
            .into_iter()
            .map(|topic| topic.destination())
            .collect();

        // Remove workers whose streams are no longer active
        // or whose tasks have unexpectedly exited.
        let obsolete: Vec<(String, usize)> = workers
            .iter()
            .filter(|((stream, _), worker)| {
                !streams.contains(stream) || worker.handle.is_finished()
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in obsolete {
            if let Some(worker) = workers.remove(&key) {
                worker.token.cancel();

                if let Err(err) = worker.handle.await {
                    warn!(
                        stream = %key.0,
                        worker_index = key.1,
                        error = %err,
                        "Outbound worker join failed"
                    );
                }
            }
        }

        // Ensure the consumer group exists before spawning workers.
        for stream in streams {
            if shutdown.is_cancelled() {
                break;
            }

            let missing_workers: Vec<usize> = (0..self.config.workers_per_stream)
                .filter(|index| !workers.contains_key(&(stream.clone(), *index)))
                .collect();

            if missing_workers.is_empty() {
                continue;
            }

            self.queue
                .ensure_group(&stream, &self.config.consumer_group)
                .await?;

            for worker_index in missing_workers {
                let token = shutdown.child_token();

                let supervisor = self.clone();
                let worker_stream = stream.clone();
                let worker_token = token.clone();

                let handle = tokio::spawn(async move {
                    supervisor
                        .send_loop(worker_stream, worker_index, worker_token)
                        .await;
                });

                workers.insert(
                    (stream.clone(), worker_index),
                    StreamWorker { token, handle },
                );
            }
        }

        Ok(())
    }

    async fn send_loop(
        self: Arc<Self>,
        stream: String,
        worker_index: usize,
        token: CancellationToken,
    ) {
        // Each consumer in a Redis consumer group needs a unique name.
        let consumer_name = format!("{}-{}", self.config.consumer_name, worker_index);

        while !token.is_cancelled() {
            let consume = self.queue.consume(
                &stream,
                &self.config.consumer_group,
                &consumer_name,
                self.config.batch_size,
                1000,
            );

            let deliveries = tokio::select! {
                _ = token.cancelled() => break,
                result = consume => {
                    match result {
                        Ok(deliveries) => deliveries,
                        Err(err) => {
                            warn!(
                                stream = %stream,
                                consumer = %consumer_name,
                                error = %err,
                                "Consume failed"
                            );

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
                sleep_or_cancel(&token, self.config.idle_interval).await;

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

                if let Err(err) = self.delivery.deliver_event(&event).await {
                    warn!(
                        event_id = %event.event_id,
                        error = %err,
                        "Event processing failed"
                    );

                    // Leave the source message pending.
                    continue;
                }

                if let Err(err) = self
                    .queue
                    .ack(&stream, &self.config.consumer_group, &delivery.delivery_id)
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
    }
}
