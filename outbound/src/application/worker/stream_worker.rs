use std::{collections::HashMap, format, sync::Arc, time::Duration};

use streamforge::traits::Queue;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, field, info, info_span, warn, Instrument, Span};
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
            while !token.is_cancelled() {
                sleep_or_cancel(&token, Duration::from_secs(10)).await;
                let batch_span = info_span!(
                    "process_stream_batch",
                    worker_id = %id,
                    stream = %stream,
                    consumer = %consumer_name,
                    batch_size = field::Empty,
                );
                async {
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
                            warn!(
                                stream = %stream,
                                consumer = %consumer_name,
                                error = %err,
                                "Consume failed"
                            );

                            sleep_or_cancel(&token, Duration::from_secs(1)).await;

                            return;
                        }
                    };
                    Span::current().record("batch_size", deliveries.len());
                    if deliveries.is_empty() {
                        sleep_or_cancel(&token, idle_interval).await;
                        return;
                    }
                    // we will referesh the destinations for each batch
                    let destinations :Arc<Vec<_> >= match projections
                        .resolve_topic_destinations(tenant_id, topic_id)
                        .await
                    {
                        Ok(d) => Arc::new( d.into_iter().map(Arc::new).collect()),
                        Err(e) => {
                            error!(err=%e,"Error while fetching destinations for topic");
                            return;
                        }
                    };
                    let destination_id_to_destinations =
                        destinations
                            .clone()
                            .iter()
                            .fold(HashMap::new(), |mut acc, item| {
                                acc.insert(item.id, item.clone());
                                acc
                            });
                    let mut join_set = JoinSet::new();

                    for delivery in deliveries {

                        if token.is_cancelled() {
                            break;
                        }
                        let queue = queue.clone();
                        let stream = stream.clone();
                        let config = config.clone(); 
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
                                    return;
                                }
                            };
                        let event_destinations = if event.destinations.is_none() {
                            destinations.clone()
                        } else {
                            Arc::new(event
                                .destinations.as_ref()
                                .unwrap()
                                .iter()
                                .map(|destination_id| {
                                   if let Some(d) = destination_id_to_destinations.get(destination_id){
                                    return Some(d);
                                   } else {
                                        warn!("event have destination specified which is not currently available");
                                        return None;
                                   }
                                })
                                .filter(|d|d.is_some())
                                .map(|d|d.unwrap().clone())
                                .collect())
                        };
                        let mut local_join_set = JoinSet::new();
                        for ed in event_destinations.as_ref(){
                            let delivery_service = delivery_service.clone();
                            // we will create the delivery request
                            let request = DeliveryRequest {
                                delivery_id: DeliveryService::delivery_id(event.event_id, ed.id),
                                event_id: event.event_id,
                                tenant_id: event.tenant_id,
                                topic_id: event.topic_id,
                                destination_id: ed.id,
                                payload: event.payload.clone(),
                                destination_config: ed.clone(),
                            };
                        local_join_set.spawn(async move {
                            if let Err(e)= delivery_service.register_delivery(&request).await{
                                warn!(error=%e,"Error while registering the delivery");
                                return ;
                            }
                            if let Err(e) = delivery_service.execute_delivery(request).await{
                                warn!(error=%e,"Error while registering the delivery");
                                return ;
                            }
                        });
                    }
                        join_set.spawn(async move {
                        while let Some(res) = local_join_set.join_next().await {
                        if let Err(err) = res {
                            warn!(error=%err,"Worker task panicked or failed");
                            return;
                        }
                    }
                    if let Err(err) = queue
                                .ack(&stream, &config.consumer_group, &delivery.delivery_id)
                                .await
                            {
                                warn!(
                                    stream = %stream,
                                    queue_message_id = %delivery.delivery_id,
                                    error = %err,
                                    "Source event ACK failed"
                                );
                                return;
                            }

                    });
                    }
                    while let Some(res) = join_set.join_next().await {
                        if let Err(err) = res {
                            warn!(error=%err,"Worker task panicked or failed")
                        }
                    }
                }
                .instrument(batch_span)
                .await;
            }

            info!(
                stream = %stream,
                consumer = %consumer_name,
                "Outbound worker stopped"
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
