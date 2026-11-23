use std::{collections::HashMap, sync::Arc, time::Duration};

use streamforge::traits::Queue;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, field, info, warn, Instrument, Span};
use uuid::Uuid;

use crate::{
    application::{
        delivery::{model::DeliveryRequest, service::DeliveryService},
        worker::util::sleep_or_cancel,
    },
    config::worker::StreamWorkerConfig,
    platform::projection_store::ProjectionStore,
};

pub struct StreamWorker {
    id: Uuid,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl StreamWorker {
    pub fn run(
        id: Uuid,
        tenant_id: Uuid,
        topic_id: Uuid,
        stream: String,
        queue: Arc<dyn Queue>,
        delivery_service: Arc<DeliveryService>,
        projections: Arc<ProjectionStore>,
        config: Arc<StreamWorkerConfig>,
        token: CancellationToken,
    ) -> Self {
        let token_clone = token.clone();
        let handle = tokio::spawn(async move {
            // Each consumer in a Redis consumer group needs a unique name.
            let consumer_name = Arc::new(format!("{}_{}", config.consumer_name_prefix, id));
            let idle_interval = Duration::from_secs(config.idle_interval_secs);

            info!(
                worker_id = %id,
                tenant_id = %tenant_id,
                topic_id = %topic_id,
                stream = %stream,
                group = %config.consumer_group,
                consumer = %consumer_name,
                "Stream worker started"
            );

            while !token.is_cancelled() {
                sleep_or_cancel(&token, Duration::from_secs(10)).await;
                if token.is_cancelled() {
                    break;
                }

                let deliveries = match queue
                    .consume(
                        &stream,
                        &config.consumer_group,
                        consumer_name.clone().as_str(),
                        config.batch_size,
                        1000,
                    )
                    .await
                {
                    Ok(deliveries) => deliveries,
                    Err(err) => {
                        // warn!(
                        //     worker_id = %id,
                        //     tenant_id = %tenant_id,
                        //     topic_id = %topic_id,
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
                    "stream.process_batch",
                    worker_id = %id,
                    tenant_id = %tenant_id,
                    topic_id = %topic_id,
                    stream = %stream,
                    consumer = %consumer_name,
                    batch_size = deliveries.len(),
                    destinations_resolved = field::Empty,
                );

                let projections = projections.clone();
                let queue = queue.clone();
                let delivery_service = delivery_service.clone();
                let config = config.clone();
                let stream_name = stream.clone();
                let consumer = consumer_name.clone();
                let cancel_token = token.clone();

                async {
                    // Refresh destinations once per batch
                    let destinations: Arc<Vec<_>> = match projections
                        .resolve_topic_destinations(tenant_id, topic_id)
                        .await
                    {
                        Ok(d) => Arc::new(d.into_iter().map(Arc::new).collect()),
                        Err(e) => {
                            error!(err = %e, "Error while fetching destinations for topic");
                            return;
                        }
                    };

                    Span::current().record("destinations_resolved", destinations.len());

                    let destination_id_to_destinations: HashMap<_, _> = destinations
                        .iter()
                        .cloned()
                        .map(|d| (d.id, d))
                        .collect();

                    let mut join_set = JoinSet::new();

                    for delivery in deliveries {
                        if cancel_token.is_cancelled() {
                            break;
                        }

                        let queue = queue.clone();
                        let stream = stream_name.clone();
                        let config = config.clone();
                        let delivery_service = delivery_service.clone();

                        let event: iam_contracts::events::PublishedEvent =
                            match serde_json::from_slice(&delivery.message.payload) {
                                Ok(event) => event,
                                Err(err) => {
                                    warn!(
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

                        let event_span = tracing::info_span!(
                            "stream.process_event",
                            queue_message_id = %delivery.delivery_id,
                            event_id = %event.event_id,
                            tenant_id = %event.tenant_id,
                            topic_id = %event.topic_id,
                            destination_count = field::Empty,
                            acked = field::Empty,
                        );

                        let event_destinations = match &event.destinations {
                            None => destinations.clone(),
                            Some(target_ids) => Arc::new(
                                target_ids
                                    .iter()
                                    .filter_map(|destination_id| {
                                        if let Some(d) = destination_id_to_destinations.get(destination_id) {
                                            Some(d.clone())
                                        } else {
                                            event_span.in_scope(|| {
                                                warn!(
                                                    destination_id = %destination_id,
                                                    "Event specifies a destination that is not currently available"
                                                );
                                            });
                                            None
                                        }
                                    })
                                    .collect(),
                            ),
                        };

                        event_span.record("destination_count", event_destinations.len());

                        let event_task_span = event_span.clone();
                        join_set.spawn(
                            async move {
                                let mut local_join_set = JoinSet::new();

                                for ed in event_destinations.as_ref() {
                                    let delivery_service = delivery_service.clone();
                                    let request = DeliveryRequest {
                                        delivery_id: DeliveryService::delivery_id(
                                            event.event_id,
                                            ed.id,
                                        ),
                                        event_id: event.event_id,
                                        tenant_id: event.tenant_id,
                                        topic_id: event.topic_id,
                                        destination_id: ed.id,
                                        payload: event.payload.clone(),
                                        destination_config: ed.clone(),
                                    };

                                    let dest_span = tracing::info_span!(
                                        "stream.deliver_to_destination",
                                        destination_id = %request.destination_id,
                                        delivery_id = %request.delivery_id,
                                    );

                                    local_join_set.spawn(
                                        async move {
                                            if let Err(e) =
                                                delivery_service.register_delivery(&request).await
                                            {
                                                warn!(error = %e, "Error while registering the delivery");
                                                return;
                                            }
                                            if let Err(e) =
                                                delivery_service.execute_delivery(request).await
                                            {
                                                warn!(error = %e, "Error while executing the delivery");
                                            }
                                        }
                                        .instrument(dest_span),
                                    );
                                }

                                while let Some(res) = local_join_set.join_next().await {
                                    if let Err(err) = res {
                                        warn!(error = %err, "Destination task panicked or failed");
                                        Span::current().record("acked", false);
                                        return;
                                    }
                                }

                                match queue
                                    .ack(&stream, &config.consumer_group, &delivery.delivery_id)
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
                            }
                            .instrument(event_task_span),
                        );
                    }

                    while let Some(res) = join_set.join_next().await {
                        if let Err(err) = res {
                            warn!(error = %err, "Event worker task panicked or failed");
                        }
                    }
                }
                .instrument(batch_span)
                .await;
            }

            info!(
                worker_id = %id,
                tenant_id = %tenant_id,
                topic_id = %topic_id,
                stream = %stream,
                consumer = %consumer_name,
                "Stream worker stopped"
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
                stream_worker = %self.id,
                error = %err,
                "Stream worker join failed"
            );
        }
    }
}
