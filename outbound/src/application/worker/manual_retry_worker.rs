use std::{sync::Arc, time::Duration};

use streamforge::traits::Queue;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn, Instrument};
use uuid::Uuid;

use crate::{
    application::{delivery::service::DeliveryService, worker::util::sleep_or_cancel},
    config::worker::ManualRetryWorkerConfig,
    platform::redis::DeliveryStore,
};

pub struct ManualRetryWorker {
    id: Uuid,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl ManualRetryWorker {
    pub fn run(
        id: Uuid,
        stream: String,
        queue: Arc<dyn Queue>,
        store: Arc<DeliveryStore>,
        delivery_service: Arc<DeliveryService>,
        config: Arc<ManualRetryWorkerConfig>,
        token: CancellationToken,
    ) -> Self {
        let token_clone = token.clone();
        let handle = tokio::spawn(async move {
            let consumer_name = Arc::new(format!("{}_{}", config.consumer_name_prefix, id));
            let idle_interval = Duration::from_millis(config.idle_interval_millisecs);

            info!(
                worker_id = %id,
                stream = %stream,
                group = %config.consumer_group,
                consumer = %consumer_name,
                "Manual retry worker started"
            );

            while !token.is_cancelled() {
                let deliveries = match queue
                    .consume(
                        &stream,
                        &config.consumer_group,
                        consumer_name.as_str(),
                        config.batch_size,
                        100,
                    )
                    .await
                {
                    Ok(deliveries) => deliveries,
                    Err(err) => {
                        // warn!(
                        //     worker_id = %id,
                        //     stream = %stream,
                        //     consumer = %consumer_name,
                        //     error = %err,
                        //     "Consume failed"
                        // );
                        sleep_or_cancel(&token, idle_interval).await;
                        continue;
                    }
                };

                if deliveries.is_empty() {
                    sleep_or_cancel(&token, idle_interval).await;
                    continue;
                }

                let batch_span = tracing::info_span!(
                    "manual_retry.process_batch",
                    worker_id = %id,
                    stream = %stream,
                    group = %config.consumer_group,
                    consumer = %consumer_name,
                    deliveries = deliveries.len(),
                    spawned = tracing::field::Empty,
                );

                async {
                    let mut join_set = JoinSet::new();
                    let mut spawned = 0usize;

                    for delivery in deliveries {
                        if token.is_cancelled() {
                            break;
                        }

                        let queue = queue.clone();
                        let stream = stream.clone();
                        let config = config.clone();
                        let store = store.clone();
                        let delivery_service = delivery_service.clone();

                        let retry_event: iam_contracts::events::RetryEvent =
                            match serde_json::from_slice(&delivery.message.payload) {
                                Ok(event) => event,
                                Err(err) => {
                                    warn!(
                                        queue_message_id = %delivery.delivery_id,
                                        error = %err,
                                        "Invalid retry event"
                                    );
                                    continue;
                                }
                            };

                        let msg_span = tracing::info_span!(
                            "manual_retry.process_one",
                            queue_message_id = %delivery.delivery_id,
                            tenant_id = %retry_event.tenant_id,
                            destination_id = %retry_event.destination_id,
                            delivery_id = %retry_event.delivery_id,
                            outcome = tracing::field::Empty,
                        );

                        spawned += 1;

                        let task_fut = async move {
                            let current_span = tracing::Span::current();

                            let request = match store
                                .as_ref()
                                .get_delivery_request(
                                    retry_event.tenant_id,
                                    retry_event.destination_id,
                                    retry_event.delivery_id,
                                )
                                .await
                            {
                                Ok(Some(request)) => request,
                                Ok(None) => {
                                    current_span.record("outcome", "missing_request");
                                    warn!("Retry request is missing from Redis");
                                    return;
                                }
                                Err(error) => {
                                    current_span.record("outcome", "store_error");
                                    error!(error = %error, "Failed to load retry request");
                                    return;
                                }
                            };

                            match delivery_service.retry_delivery_once(request).await {
                                Ok(()) => {
                                    if let Err(err) = queue
                                        .ack(&stream, &config.consumer_group, &delivery.delivery_id)
                                        .await
                                    {
                                        current_span.record("outcome", "ack_failed");
                                        warn!(error = %err, "Source event ACK failed");
                                        return;
                                    }
                                    current_span.record("outcome", "retried_and_acked");
                                    debug!("Manual retry succeeded and acked");
                                }
                                Err(error) => {
                                    current_span.record("outcome", "retry_failed");
                                    error!(error = %error, "Retry execution failed");
                                }
                            }
                        }
                        .instrument(msg_span);

                        join_set.spawn(task_fut);
                    }

                    tracing::Span::current().record("spawned", spawned);

                    while let Some(res) = join_set.join_next().await {
                        if let Err(err) = res {
                            warn!(error = %err, "Worker task panicked or failed");
                        }
                    }
                }
                .instrument(batch_span)
                .await;
            }

            info!(
                worker_id = %id,
                stream = %stream,
                consumer = %consumer_name,
                "Manual retry worker stopped"
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
                manual_retry_worker = %self.id,
                error = %err,
                "Manual retry worker join failed"
            );
        }
    }
}
