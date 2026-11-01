use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use iam_contracts::{events::PublishedEvent, streams::DELIVERY_RESULTS};
use streamforge::{message::Message, traits::Queue};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::application::delivery::service::DeliveryService;
use iam_contracts::topic_registory::TopicRegistry;

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub consumer_group: String,
    pub consumer_name: String,
    pub batch_size: usize,
    pub registry_refresh_interval: Duration,
    pub idle_interval: Duration,
}

pub struct OuboundWorker {
    token: CancellationToken,
    handle: JoinHandle<()>,
}

async fn sleep_or_cancel(token: &CancellationToken, d: Duration) {
    tokio::select! {
        _ = token.cancelled() => {}
        _ = tokio::time::sleep(d) => {}
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
        let mut workers: HashMap<String, OuboundWorker> = HashMap::new();
        while !shutdown.is_cancelled() {
            match self.reconcile(&mut workers, &shutdown).await {
                Ok(()) => {}
                Err(e) => warn!( error = %e, "Outbound reconcile failed"),
            }
            sleep_or_cancel(&shutdown, self.config.registry_refresh_interval).await;
        }
    }
    async fn reconcile(
        self: &Arc<Self>,
        workers: &mut HashMap<String, OuboundWorker>,
        shutdown: &CancellationToken,
    ) -> Result<()> {
        let wanted = self
            .registry
            .list_active()
            .await?
            .into_iter()
            .map(|x| x.destination())
            .collect::<HashSet<_>>();
        workers.retain(|name, w| {
            if !wanted.contains(name) {
                info!(stream = %name, "stream no longer active, stopping worker");
                w.token.cancel();
                return false;
            }
            if w.handle.is_finished() {
                warn!(stream = %name, "woker exited unexpectedly, will restart");
                return false;
            }
            true
        });
        for name in wanted {
            if workers.contains_key(&name) {
                continue;
            }
            if let Err(e) = self
                .queue
                .ensure_group(&name, &self.config.consumer_group)
                .await
            {
                warn!(stream = %name, error = %e, "ensure_group failed, retrying next tick");
                continue;
            }
            info!(stream = %name, group = %&&self.config.consumer_group, "starting worker");
            let token = shutdown.child_token();
            let handle = tokio::spawn(self.clone().send_loop(name.clone(), token.clone()));
            workers.insert(name, OuboundWorker { token, handle });
        }
        Ok(())
    }
    async fn send_loop(self: Arc<Self>, stream: String, token: CancellationToken) {
        while !token.is_cancelled() {
            let deliveries = match self
                .queue
                .consume(
                    &stream,
                    &self.config.consumer_group,
                    &self.config.consumer_name,
                    self.config.batch_size,
                    0,
                )
                .await
            {
                Ok(val) => val,
                Err(e) => {
                    warn!( stream = %stream, error = %e, "Outbound consume failed");
                    sleep_or_cancel(&token, Duration::from_secs(1)).await;
                    continue;
                }
            };
            for delivery in deliveries {
                let event: PublishedEvent = match serde_json::from_slice(&delivery.message.payload)
                {
                    Ok(event) => event,
                    Err(err) => {
                        // Do not silently discard malformed messages.
                        // Leave this entry pending for later recovery or
                        // explicit dead-letter handling.
                        warn!(
                            stream = %stream,
                            delivery_id = %delivery.delivery_id,
                            error = %err,
                            "invalid published event"
                        );
                        continue;
                    }
                };

                let outcomes = match self.delivery.deliver_event(&event).await {
                    Ok(outcomes) => outcomes,
                    Err(err) => {
                        warn!(
                            event_id = %event.event_id,
                            error = %err,
                            "event delivery processing failed"
                        );
                        sleep_or_cancel(&token, Duration::from_millis(200)).await;
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

                if let Err(e) = self
                    .queue
                    .ack(&stream, &self.config.consumer_group, &delivery.delivery_id)
                    .await
                {
                    warn!( stream = %stream, error = %e, " Outbound ack failed, will be redelivered");
                }

                info!(
                    event_id = %event.event_id,
                    delivery_id = %delivery.delivery_id,
                    "published event processed"
                );
            }
        }
    }
}
