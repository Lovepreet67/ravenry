use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use iam_contracts::{delivery_outcome::EventDestinationError, streams};
use streamforge::{traits::Queue, Message};
use tokio::{
    sync::mpsc,
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, field, info, warn, Instrument, Span};
use uuid::Uuid;

use crate::{
    application::delivery::{model::DeliveryRequest, service::DeliveryService},
    config::worker::StreamWorkerConfig,
    platform::{projection_store::ProjectionStore, redis::DeliveryStore},
};

use super::runner::{WorkerChannels, WorkerRunner, WorkerTask};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamAssignment {
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
    pub stream: String,
}

#[derive(Debug, Clone)]
pub enum StreamWorkerReport {
    Idle { worker_id: Uuid, idle_passes: u32 },
}

pub struct StreamWorker {
    id: Uuid,
    consumer_name: String,
    queue: Arc<dyn Queue>,
    projections: Arc<ProjectionStore>,
    delivery_store: Arc<DeliveryStore>,
    config: Arc<StreamWorkerConfig>,
    token: CancellationToken,
}

pub struct StreamWorkerHandle {
    pub id: Uuid,
    pub add_tx: mpsc::Sender<StreamAssignment>,
    pub deprecate_tx: mpsc::Sender<StreamAssignment>,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl StreamWorkerHandle {
    pub fn spawn(
        id: Uuid,
        queue: Arc<dyn Queue>,
        projections: Arc<ProjectionStore>,
        delivery_store: Arc<DeliveryStore>,
        config: Arc<StreamWorkerConfig>,
        report_tx: mpsc::Sender<StreamWorkerReport>,
        channel_cap: usize,
        token: CancellationToken,
    ) -> Self {
        let (add_tx, add_rx) = mpsc::channel(channel_cap);
        let (deprecate_tx, deprecate_rx) = mpsc::channel(channel_cap);
        let idle_sleep = Duration::from_secs(config.idle_interval_secs);
        let consumer_name = format!("{}_{}", config.consumer_name_prefix, id);

        let worker = StreamWorker {
            id,
            consumer_name: consumer_name.clone(),
            queue,
            projections,
            delivery_store,
            config: config.clone(),
            token: token.clone(),
        };

        let channels = WorkerChannels {
            add_rx,
            deprecate_rx,
            report_tx,
        };

        let token_clone = token.clone();
        let group = config.consumer_group.clone();
        let handle = tokio::spawn(async move {
            info!(
                worker_id = %id,
                group = %group,
                consumer = %consumer_name,
                "Stream worker started"
            );

            WorkerRunner::new(worker, channels, idle_sleep, token_clone)
                .run()
                .await;

            info!(
                worker_id = %id,
                group = %group,
                consumer = %consumer_name,
                "Stream worker stopped"
            );
        });

        Self {
            id,
            add_tx,
            deprecate_tx,
            token,
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

#[async_trait]
impl WorkerTask for StreamWorker {
    type Assignment = StreamAssignment;
    type Report = StreamWorkerReport;

    async fn step(&mut self, assignment: &Self::Assignment) -> Result<bool, anyhow::Error> {
        let batch_span = tracing::info_span!(
            "stream.process_batch",
            worker_id = %self.id,
            tenant_id = %assignment.tenant_id,
            topic_id = %assignment.topic_id,
            stream = %assignment.stream,
            consumer = %self.consumer_name,
            acked = tracing::field::Empty,
            batch_size = tracing::field::Empty,
            destinations_resolved = field::Empty,
        );

        let queue = self.queue.clone();
        let projections = self.projections.clone();
        let config = self.config.clone();
        let token = self.token.clone();
        let consumer_name = self.consumer_name.clone();

        async move {
            let deliveries = match queue
                .consume(
                    &assignment.stream,
                    &config.consumer_group,
                    consumer_name.as_str(),
                    config.batch_size,
                    20,
                )
                .await
            {
                Ok(deliveries) => deliveries,
                Err(err) => {
                    return Err(anyhow::anyhow!("consume failed: {err}"));
                }
            };

            Span::current().record("batch_size", deliveries.len());

            if deliveries.is_empty() {
                return Ok(false);
            }

            let destinations: Arc<Vec<_>> = match projections
                .resolve_topic_destinations(assignment.tenant_id, assignment.topic_id)
                .await
            {
                Ok(d) => Arc::new(d.into_iter().map(Arc::new).collect()),
                Err(e) => {
                    error!(err = %e, "Error while fetching destinations for topic");
                    return Err(anyhow::anyhow!("resolve_topic_destinations failed: {e}"));
                }
            };

            Span::current().record("destinations_resolved", destinations.len());

            let destination_id_to_destinations: HashMap<_, _> =
                destinations.iter().cloned().map(|d| (d.id, d)).collect();
            let mut destination_id_to_deliveries: HashMap<Uuid, Vec<Message>> = HashMap::default();
            let mut invalid_events = Vec::new();
            let mut destination_id_to_invalid_deliveries: HashMap<Uuid, Vec<Message>> =
                HashMap::default();

            for delivery in &deliveries {
                if token.is_cancelled() {
                    break;
                }

                let event: iam_contracts::events::PublishedEvent =
                    match serde_json::from_slice(&delivery.message.payload) {
                        Ok(event) => event,
                        Err(err) => {
                            warn!(
                                queue_message_id = %delivery.delivery_id,
                                error = %err,
                                "Invalid published event"
                            );
                            invalid_events.push(delivery.message.clone());
                            continue;
                        }
                    };

                let event_destinations = match &event.destinations {
                    None => destinations.clone(),
                    Some(target_ids) => Arc::new(
                        target_ids
                            .iter()
                            .filter_map(|destination_id| {
                                destination_id_to_destinations.get(destination_id).cloned()
                            })
                            .collect(),
                    ),
                };

                for ed in event_destinations.as_ref() {
                    let request = DeliveryRequest {
                        delivery_id: DeliveryService::delivery_id(event.event_id, ed.id),
                        event_id: event.event_id,
                        tenant_id: event.tenant_id,
                        topic_id: event.topic_id,
                        destination_id: ed.id,
                        payload: event.payload.clone(),
                        destination_config: ed.clone(),
                        curr_attempt: None,
                    };
                    let delivery_req_serialized = match serde_json::to_vec(&request) {
                        Ok(v) => v,
                        Err(e) => {
                            let event_destination_error = EventDestinationError {
                                event_id: event.event_id,
                                tenant_id: event.tenant_id,
                                topic_id: event.topic_id,
                                destination_id: ed.id,
                                error: e.to_string(),
                            };

                            let message = Message::new(
                                serde_json::to_vec(&event_destination_error).expect("Can't fail"),
                            );
                            destination_id_to_invalid_deliveries
                                .entry(ed.id)
                                .or_default()
                                .push(message);
                            continue;
                        }
                    };
                    let delivery_message = Message::new(delivery_req_serialized)
                        .with_id(DeliveryService::delivery_id(event.event_id, ed.id))
                        .with_header("message_type", "delivery");
                    destination_id_to_deliveries
                        .entry(ed.id)
                        .or_default()
                        .push(delivery_message);
                }
            }

            let mut join_set = JoinSet::new();
            self.delivery_store
                .mark_destinations_active(
                    destination_id_to_deliveries
                        .iter()
                        .map(|(destination_id, _)| (assignment.tenant_id, destination_id.clone()))
                        .collect::<Vec<(Uuid, Uuid)>>()
                        .as_slice(),
                )
                .await?;

            for (destination_id, dest_deliveries) in destination_id_to_deliveries {
                let queue = queue.clone();
                let tenant_id = assignment.tenant_id;
                join_set.spawn(async move {
                    let queue_key = streams::destination_deliveries(tenant_id, destination_id);
                    queue
                        .publish_batch(
                            dest_deliveries
                                .into_iter()
                                .map(|message| (queue_key.as_str(), message))
                                .collect(),
                        )
                        .await
                });
            }

            if !invalid_events.is_empty() {
                let queue_t = queue.clone();
                let tenant_id = assignment.tenant_id;
                let topic_id = assignment.topic_id;
                join_set.spawn(async move {
                    let event_dlq_key = streams::topic_events_dlq(tenant_id, topic_id);
                    queue_t
                        .publish_batch(
                            invalid_events
                                .into_iter()
                                .map(|message| (event_dlq_key.as_str(), message))
                                .collect(),
                        )
                        .await
                });
            }

            for (destination_id, dest_deliveries) in destination_id_to_invalid_deliveries {
                let queue = queue.clone();
                let tenant_id = assignment.tenant_id;
                join_set.spawn(async move {
                    let queue_key = streams::destination_dlq(tenant_id, destination_id);
                    queue
                        .publish_batch(
                            dest_deliveries
                                .into_iter()
                                .map(|message| (queue_key.as_str(), message))
                                .collect(),
                        )
                        .await
                });
            }

            let mut tasks_failed = 0usize;

            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(Ok(_)) => {}
                    Ok(Err(err)) => {
                        tasks_failed += 1;
                        warn!(error = %err, "Publish batch failed");
                    }
                    Err(join_err) => {
                        tasks_failed += 1;
                        warn!(error = %join_err, "Event worker task panicked or failed");
                    }
                }
            }

            if tasks_failed > 0 {
                Span::current().record("acked", false);
                warn!(
                    failed_tasks = tasks_failed,
                    "Skipping acknowledgement as some publish tasks failed"
                );
                return Ok(true);
            }

            match queue
                .ack_batch(
                    &assignment.stream,
                    &config.consumer_group,
                    deliveries
                        .iter()
                        .map(|d| d.delivery_id.as_str())
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
                .await
            {
                Ok(()) => {
                    Span::current().record("acked", true);
                    debug!("Source event ACKed");
                }
                Err(err) => {
                    Span::current().record("acked", false);
                    warn!(error = %err, "Source event ACK failed");
                }
            }

            Ok(true)
        }
        .instrument(batch_span)
        .await
    }

    fn on_idle(&mut self, idle_passes: u32) -> Option<Self::Report> {
        return None;
        // Some(StreamWorkerReport::Idle {
        //     worker_id: self.id,
        //     idle_passes,
        // })
    }
}
