use std::{sync::Arc, time::Duration};

use iam_contracts::streams;
use streamforge::{traits::Queue, Message};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn, Instrument};
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
                dlq_worker = %self.id,
                error = %err,
                "DLQ worker join failed"
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
            let queue_destination = streams::destination_dlq(tenant_id, destination_id);

            info!(
                worker_id = %id,
                tenant_id = %tenant_id,
                destination_id = %destination_id,
                queue_destination = %queue_destination,
                "DLQ worker started"
            );

            while !shutdown_token.is_cancelled() {
                let messages = match delivery_store
                    .get_pending_dlq_messages(tenant_id, destination_id, config.batch_size)
                    .await
                {
                    Ok(messages) => messages,
                    Err(error) => {
                        // error!(
                        //     worker_id = %id,
                        //     tenant_id = %tenant_id,
                        //     destination_id = %destination_id,
                        //     error = %error,
                        //     "Failed to load pending DLQ messages"
                        // );
                        sleep_or_cancel(&shutdown_token, poll_interval).await;
                        continue;
                    }
                };

                if messages.is_empty() {
                    sleep_or_cancel(&shutdown_token, poll_interval).await;
                    continue;
                }

                let batch_span = tracing::info_span!(
                    "dlq.process_batch",
                    worker_id = %id,
                    tenant_id = %tenant_id,
                    destination_id = %destination_id,
                    queue_destination = %queue_destination,
                    batch_len = messages.len(),
                );

                async {
                    for (delivery_id, dlq_message) in messages {
                        let msg_span = tracing::debug_span!(
                            "dlq.publish_message",
                            delivery_id = %delivery_id,
                            outcome = tracing::field::Empty,
                        );

                        let payload = match serde_json::to_vec(&dlq_message) {
                            Ok(payload) => payload,
                            Err(error) => {
                                msg_span.record("outcome", "serialize_failed");
                                msg_span.in_scope(|| {
                                    error!(error = %error, "Failed to serialize DLQ message");
                                });
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

                        match queue
                            .publish(&queue_destination, message)
                            .instrument(msg_span.clone())
                            .await
                        {
                            Ok(_) => {
                                match delivery_store
                                    .acknowledge_dlq_message(
                                        tenant_id,
                                        destination_id,
                                        delivery_id,
                                    )
                                    .instrument(msg_span.clone())
                                    .await
                                {
                                    Ok(()) => {
                                        msg_span.record("outcome", "acked");
                                        debug!(parent: &msg_span, "DLQ message published and acked");
                                    }
                                    Err(error) => {
                                        msg_span.record("outcome", "ack_failed");
                                        msg_span.in_scope(|| {
                                            error!(
                                                error = %error,
                                                "DLQ published but Redis acknowledgement failed"
                                            );
                                        });
                                    }
                                }
                            }
                            Err(error) => {
                                msg_span.record("outcome", "publish_failed");
                                msg_span.in_scope(|| {
                                    warn!(
                                        error = %error,
                                        "DLQ publication failed; message remains pending"
                                    );
                                });
                            }
                        }
                    }
                }
                .instrument(batch_span)
                .await;

                sleep_or_cancel(&shutdown_token, poll_interval).await;
            }

            info!(
                worker_id = %id,
                tenant_id = %tenant_id,
                destination_id = %destination_id,
                "DLQ worker stopped"
            );
        });

        Self { id, token, handle }
    }
}
