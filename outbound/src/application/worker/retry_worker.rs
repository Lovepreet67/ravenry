use std::{sync::Arc, time::Duration};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};
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
            while !shutdown_token.is_cancelled() {
                let delivery_ids = match store
                    .claim_due_retries(tenant_id, destination_id, config.batch_size)
                    .await
                {
                    Ok(ids) => ids,
                    Err(error) => {
                        error!(
                            tenant_id = %tenant_id,
                            destination_id = %destination_id,
                            error = %error,
                            "Failed to discover due retries"
                        );
                        continue;
                    }
                };

                for delivery_id in delivery_ids {
                    let request = match store
                        .as_ref()
                        .get_delivery_request(tenant_id, destination_id, delivery_id)
                        .await
                    {
                        Ok(Some(request)) => request,

                        Ok(None) => {
                            warn!(
                                tenant_id = %tenant_id,
                                destination_id = %destination_id,
                                delivery_id = %delivery_id,
                                "Retry request is missing from Redis"
                            );
                            continue;
                        }

                        Err(error) => {
                            error!(
                                delivery_id = %delivery_id,
                                error = %error,
                                "Failed to load retry request"
                            );
                            continue;
                        }
                    };

                    match delivery_service.retry_delivery(request).await {
                        Ok(()) => {
                            debug!(
                                delivery_id = %delivery_id,
                                "Retry execution completed"
                            );
                        }
                        Err(error) => {
                            error!(
                                delivery_id = %delivery_id,
                                error = %error,
                                "Retry execution failed"
                            );
                        }
                    }
                    sleep_or_cancel(&shutdown_token, poll_interval).await;
                }
            }
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
                stream_worker = %self.id,
                error = %err,
                "Stream worker join failed"
            );
        }
    }
}
