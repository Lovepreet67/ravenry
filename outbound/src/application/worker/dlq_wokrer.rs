use std::{sync::Arc, time::Duration};

use iam_contracts::streams;
use streamforge::{traits::Queue, Message};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    application::worker::util::sleep_or_cancel, config::worker::DlqWorkerConfig,
    platform::redis::delivery_store::DeliveryStore,
};

pub struct DlqWorker {
    id: Uuid,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl DlqWorker {
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
    pub fn run(
        id: Uuid,
        tenant_id: Uuid,
        destination_id: Uuid,
        queue: Arc<dyn Queue>,
        config: Arc<DlqWorkerConfig>,
        delivery_store: Arc<DeliveryStore>,
        shutdown_token: CancellationToken,
    ) -> Self {
        let token = shutdown_token.clone();
        let handle = tokio::task::spawn(async move {
            let poll_interval = Duration::from_millis(config.poll_interval_millis);
            while !shutdown_token.is_cancelled() {
                let messages = match delivery_store
                    .get_pending_dlq_messages(tenant_id, destination_id, config.batch_size)
                    .await
                {
                    Ok(messages) => messages,
                    Err(error) => {
                        error!(
                            tenant_id = %tenant_id,
                            destination_id = %destination_id,
                            error = %error,
                            "Failed to load pending DLQ messages"
                        );
                        continue;
                    }
                };

                let queue_destination = streams::destination_dlq(tenant_id, destination_id);

                for (delivery_id, dlq_message) in messages {
                    let payload = match serde_json::to_vec(&dlq_message) {
                        Ok(payload) => payload,
                        Err(error) => {
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
                        .with_header("tenant_id", tenant_id.to_string())
                        .with_header("destination_id", destination_id.to_string());

                    match queue.publish(&queue_destination, message).await {
                        Ok(_) => {
                            if let Err(error) = delivery_store
                                .acknowledge_dlq_message(tenant_id, destination_id, delivery_id)
                                .await
                            {
                                error!(
                                    delivery_id = %delivery_id,
                                    error = %error,
                                    "DLQ published but Redis acknowledgement failed"
                                );
                                continue;
                            }
                        }

                        Err(error) => {
                            warn!(
                                delivery_id = %delivery_id,
                                error = %error,
                                "DLQ publication failed; message remains pending"
                            );
                        }
                    }
                    sleep_or_cancel(&shutdown_token, poll_interval).await;
                }
            }
        });
        Self { id, token, handle }
    }
}
