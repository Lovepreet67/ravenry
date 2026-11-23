use std::{sync::Arc, time::Duration};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn, Instrument};
use uuid::Uuid;

use crate::{
    application::{delivery::service::DeliveryService, worker::util::sleep_or_cancel},
    config::worker::RetryWorkerConfig,
    platform::redis::DeliveryStore,
};

pub struct RetryWorker {
    id: Uuid,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl RetryWorker {
    pub fn run(
        id: Uuid,
        tenant_id: Uuid,
        destination_id: Uuid,
        store: Arc<DeliveryStore>,
        delivery_service: Arc<DeliveryService>,
        config: Arc<RetryWorkerConfig>,
        shutdown_token: CancellationToken,
    ) -> Self {
        let token = shutdown_token.clone();
        let handle = tokio::spawn(async move {
            let poll_interval = Duration::from_millis(config.poll_interval_millis);

            info!(
                worker_id = %id,
                tenant_id = %tenant_id,
                destination_id = %destination_id,
                "Retry worker started"
            );

            while !shutdown_token.is_cancelled() {
                sleep_or_cancel(&shutdown_token, poll_interval).await;
                if shutdown_token.is_cancelled() {
                    break;
                }

                let delivery_ids = match store
                    .claim_due_retries(tenant_id, destination_id, config.batch_size)
                    .await
                {
                    Ok(ids) => ids,
                    Err(error) => {
                        error!(
                            worker_id = %id,
                            tenant_id = %tenant_id,
                            destination_id = %destination_id,
                            error = %error,
                            "Failed to discover due retries"
                        );
                        continue;
                    }
                };

                if delivery_ids.is_empty() {
                    continue;
                }

                let batch_span = tracing::info_span!(
                    "retry.process_batch",
                    worker_id = %id,
                    tenant_id = %tenant_id,
                    destination_id = %destination_id,
                    batch_len = delivery_ids.len(),
                );

                async {
                    for delivery_id in delivery_ids {
                        if shutdown_token.is_cancelled() {
                            break;
                        }

                        let msg_span = tracing::info_span!(
                            "retry.process_one",
                            delivery_id = %delivery_id,
                            outcome = tracing::field::Empty,
                        );

                        async {
                            let current_span = tracing::Span::current();

                            let request = match store
                                .as_ref()
                                .get_delivery_request(tenant_id, destination_id, delivery_id)
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

                            match delivery_service.retry_delivery(request).await {
                                Ok(()) => {
                                    current_span.record("outcome", "retried");
                                    debug!("Retry execution completed");
                                }
                                Err(error) => {
                                    current_span.record("outcome", "retry_failed");
                                    error!(error = %error, "Retry execution failed");
                                }
                            }
                        }
                        .instrument(msg_span)
                        .await;
                    }
                }
                .instrument(batch_span)
                .await;
            }

            info!(
                worker_id = %id,
                tenant_id = %tenant_id,
                destination_id = %destination_id,
                "Retry worker stopped"
            );
        });

        Self { id, token, handle }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub async fn cancel(self) {
        self.token.cancel();
        if let Err(err) = self.handle.await {
            warn!(
                retry_worker = %self.id,
                error = %err,
                "Retry worker join failed"
            );
        }
    }
}
