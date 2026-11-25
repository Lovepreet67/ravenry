use async_trait::async_trait;
use iam_contracts::{keys, streams};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::mpsc,
    task::{JoinHandle, JoinSet},
};

use streamforge::{traits::Queue, Message};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn, Instrument, Span};
use uuid::Uuid;

use crate::{
    application::{
        delivery::{model::DeliveryRequest, service::DeliveryService},
        worker::runner::{WorkerChannels, WorkerRunner, WorkerTask},
    },
    config::worker::DeliveryWorkerConfig,
    platform::redis::DeliveryStore,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryAssignment {
    pub tenant_id: Uuid,
    pub destination_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum DeliveryReport {
    Idle { worker_id: Uuid, idle_passes: u32 },
}

pub struct DeliveryWorker {
    id: Uuid,
    consumer_name: String,
    queue: Arc<dyn Queue>,
    delivery_service: Arc<DeliveryService>,
    delivery_store: Arc<DeliveryStore>,
    config: Arc<DeliveryWorkerConfig>,
    token: CancellationToken,
}

pub struct DeliveryWorkerHandle {
    pub id: Uuid,
    pub add_tx: mpsc::Sender<DeliveryAssignment>,
    pub deprecate_tx: mpsc::Sender<DeliveryAssignment>,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl DeliveryWorkerHandle {
    pub fn spawn(
        id: Uuid,
        queue: Arc<dyn Queue>,
        delivery_service: Arc<DeliveryService>,
        delivery_store: Arc<DeliveryStore>,
        config: Arc<DeliveryWorkerConfig>,
        report_tx: mpsc::Sender<DeliveryReport>,
        channel_cap: usize,
        token: CancellationToken,
    ) -> Self {
        let (add_tx, add_rx) = mpsc::channel(channel_cap);
        let (deprecate_tx, deprecate_rx) = mpsc::channel(channel_cap);
        let idle_sleep = Duration::from_secs(config.idle_interval_secs);
        let consumer_name = format!("{}_{}", config.consumer_name_prefix, id);
        let worker = DeliveryWorker {
            id,
            consumer_name: consumer_name.clone(),
            queue,
            delivery_service,
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
            info!(worker_id=%id,group=%group,consumer=%consumer_name,"Delivery Worker started");
            WorkerRunner::new(worker, channels, idle_sleep, token_clone)
                .run()
                .await;
            info!(worker_id=%id,group=%group,consumer=%consumer_name,"Delivery Worker stopped");
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
                worker_id= %self.id,
                error = %err,
                "Delivery worker join failed"
            );
        }
    }
}

#[async_trait]
impl WorkerTask for DeliveryWorker {
    type Assignment = DeliveryAssignment;
    type Report = DeliveryReport;

    async fn step(&mut self, assignment: &Self::Assignment) -> Result<bool, anyhow::Error> {
        let stream =
            streams::destination_deliveries(assignment.tenant_id, assignment.destination_id);
        let batch_span = tracing::info_span!(
            "delivery.process_batch",
            worker_id = %self.id,
            tenant_id = %assignment.tenant_id,
            destination_id = %assignment.destination_id,
            stream = %stream,
            consumer = %self.consumer_name,
            acked = tracing::field::Empty,
            batch_size = tracing::field::Empty,
        );

        let queue = self.queue.clone();
        let delivery_service = self.delivery_service.clone();
        let config = self.config.clone();
        let token = self.token.clone();
        let consumer_name = self.consumer_name.clone();

        async move {
            let deliveries = match queue
                .consume(
                    &stream,
                    &config.consumer_group,
                    consumer_name.as_str(),
                    config.batch_size,
                    1000,
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

            let mut join_set = JoinSet::new();
            let mut invalid_deliveries = Vec::new();
            let mut invalid_delivery_ids = Vec::new();
            let mut ack_ids = Vec::with_capacity(deliveries.len());
            let result_stream = Arc::new(streams::destination_outcomes(
                assignment.tenant_id,
                assignment.destination_id,
            ));
            let retry_set_key = Arc::new(keys::destination_retry(
                assignment.tenant_id,
                assignment.destination_id,
            ));

            for delivery in deliveries {
                if token.is_cancelled() {
                    break;
                }

                let request: DeliveryRequest =
                    match serde_json::from_slice(&delivery.message.payload) {
                        Ok(req) => req,
                        Err(err) => {
                            warn!(
                                queue_message_id = %delivery.delivery_id,
                                error = %err,
                                "Invalid delivery request"
                            );
                            invalid_deliveries.push(delivery.message);
                            invalid_delivery_ids.push(delivery.delivery_id);
                            continue;
                        }
                    };

                let delivery_id = delivery.delivery_id.clone();
                let delivery_service = delivery_service.clone();
                let delivery_store = self.delivery_store.clone();
                let queue = queue.clone();
                let result_stream = result_stream.clone();
                let retry_set_key = retry_set_key.clone();

                join_set.spawn(async move {
                    let outcome = delivery_service
                        .execute_delivery(request.clone())
                        .await
                        .map_err(|e| anyhow::anyhow!("execute_delivery failed: {e}"))?;

                    let payload = serde_json::to_vec(&outcome)
                        .map_err(|e| anyhow::anyhow!("serialize outcome failed: {e}"))?;

                    let message = Message::new(payload);
                    queue
                        .publish(&result_stream, message)
                        .await
                        .map_err(|e| anyhow::anyhow!("publish outcome failed: {e}"))?;

                    if outcome.will_retry {
                        let retry_at = outcome
                            .next_attempt_at
                            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::seconds(5));
                        let score_ms = retry_at.timestamp_millis();

                        let mut retry_request = request.clone();
                        retry_request.curr_attempt = Some(outcome.attempt + 1);

                        delivery_store
                            .schedule_retry(&retry_set_key, &retry_request, score_ms)
                            .await
                            .map_err(|e| anyhow::anyhow!("schedule_retry failed: {e}"))?;
                    }

                    Ok::<String, anyhow::Error>(delivery_id)
                });
            }

            if !invalid_deliveries.is_empty() {
                let dlq_key =
                    streams::destination_dlq(assignment.tenant_id, assignment.destination_id);
                if let Err(err) = queue
                    .publish_batch(
                        invalid_deliveries
                            .into_iter()
                            .map(|m| (dlq_key.as_str(), m))
                            .collect(),
                    )
                    .await
                {
                    warn!(error = %err, "Failed to publish invalid delivery requests to DLQ");
                    Span::current().record("acked", false);
                    return Ok(true);
                } else {
                    ack_ids.extend(invalid_delivery_ids);
                }
            }

            let mut tasks_failed = 0usize;
            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(Ok(delivery_id)) => {
                        ack_ids.push(delivery_id);
                    }
                    Ok(Err(err)) => {
                        tasks_failed += 1;
                        warn!(error = %err, "Delivery task failed");
                    }
                    Err(join_err) => {
                        tasks_failed += 1;
                        warn!(error = %join_err, "Delivery task panicked or was cancelled");
                    }
                }
            }

            if tasks_failed > 0 {
                warn!(
                    failed_tasks = tasks_failed,
                    acked_count = ack_ids.len(),
                    "Skipping some ACK as some delivery tasks failed"
                );
            }

            if !ack_ids.is_empty() {
                let ack_refs: Vec<&str> = ack_ids.iter().map(String::as_str).collect();
                match queue
                    .ack_batch(&stream, &config.consumer_group, ack_refs.as_slice())
                    .await
                {
                    Ok(()) => {
                        Span::current().record("acked", true);
                        debug!(acked_count = ack_refs.len(), "Delivery batch ACKed");
                    }
                    Err(err) => {
                        Span::current().record("acked", false);
                        warn!(error = %err, "Delivery batch ACK failed");
                    }
                }
            }

            Ok(true)
        }
        .instrument(batch_span)
        .await
    }

    fn on_idle(&mut self, idle_passes: u32) -> Option<Self::Report> {
        None
        // Some(DeliveryReport::Idle {
        //     worker_id: self.id,
        //     idle_passes,
        // })
    }
}
