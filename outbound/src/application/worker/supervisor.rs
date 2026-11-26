use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use iam_contracts::{streams, topic_registory::TopicRegistry};
use streamforge::traits::Queue;
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::{
    application::{
        delivery::service::DeliveryService,
        worker::{
            delivery_worker::{DeliveryAssignment, DeliveryWorkerDeps, DeliveryWorkerMaster},
            retry_worker::{RetryAssignment, RetryWorkerMaster},
            stream_worker::{StreamAssignment, StreamWorkerDeps, StreamWorkerMaster},
            util::sleep_or_cancel,
        },
    },
    config::worker::WorkerRuntimeConfig,
    platform::{projection_store::ProjectionStore, redis::DeliveryStore},
};

pub struct WorkerSupervisor {
    config: Arc<WorkerRuntimeConfig>,
    topic_registory: Arc<dyn TopicRegistry>,
    queue: Arc<dyn Queue>,
    projection_store: Arc<ProjectionStore>,
    delivery_store: Arc<DeliveryStore>,
    delivery_service: Arc<DeliveryService>,
    shutdown_token: CancellationToken,
}

impl WorkerSupervisor {
    pub fn new(
        config: Arc<WorkerRuntimeConfig>,
        topic_registory: Arc<dyn TopicRegistry>,
        delivery_service: Arc<DeliveryService>,
        projection_store: Arc<ProjectionStore>,
        delivery_store: Arc<DeliveryStore>,
        queue: Arc<dyn Queue>,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            config,
            topic_registory,
            delivery_service,
            projection_store,
            delivery_store,
            queue,
            shutdown_token,
        }
    }

    pub async fn run(self: Arc<Self>) {
        let reconcile_interval = Duration::from_secs(self.config.registry_refresh_interval_secs);
        info!("Starting workers");

        let mut stream_worker_master = StreamWorkerMaster::new(StreamWorkerDeps {
            queue: self.queue.clone(),
            delivery_store: self.delivery_store.clone(),
            projections: self.projection_store.clone(),
            config: self.config.stream_worker_config.clone(),
        });
        let mut retry_worker_master = RetryWorkerMaster::new(
            self.delivery_store.clone(),
            self.config.retry_worker_config.clone(),
        );
        let mut delivery_worker_master = DeliveryWorkerMaster::new(DeliveryWorkerDeps {
            queue: self.queue.clone(),
            config: self.config.delivery_worker_config.clone(),
            delivery_service: self.delivery_service.clone(),
            delivery_store: self.delivery_store.clone(),
        });

        while !self.shutdown_token.is_cancelled() {
            if let Err(err) = self
                .reconcile_consumer(
                    &mut stream_worker_master,
                    &mut delivery_worker_master,
                    &mut retry_worker_master,
                )
                .await
            {
                warn!(error=%err, "Worker reconcilation failed");
            }
            sleep_or_cancel(&self.shutdown_token, reconcile_interval).await;
        }

        // Stop all stream and delivery workers cleanly on shutdown
        let grace_duration = Duration::from_secs(self.config.shutdown_grace_secs);
        stream_worker_master.shutdown(grace_duration).await;
        delivery_worker_master.shutdown(grace_duration).await;
        retry_worker_master.shutdown(grace_duration).await;
    }

    #[instrument(
        skip_all,
        level = "debug",
        fields(
            active_streams = tracing::field::Empty,
            active_deliveries = tracing::field::Empty,
            active_retries = tracing::field::Empty,
            active_dlqs = tracing::field::Empty,
            active_manual_retries = tracing::field::Empty,
        )
    )]
    async fn reconcile_consumer(
        self: &Arc<Self>,
        stream_worker_master: &mut StreamWorkerMaster,
        delivery_worker_master: &mut DeliveryWorkerMaster,
        retry_worker_master: &mut RetryWorkerMaster,
    ) -> Result<()> {
        let streams: Vec<_> = self
            .topic_registory
            .list_active()
            .await?
            .into_iter()
            .collect();

        // 1. Reconcile stream workers (reports, deprecations, and new assignments)
        let stream_worker_assignement: HashSet<_> = streams
            .iter()
            .map(|stream| StreamAssignment {
                tenant_id: stream.tenant_id,
                topic_id: stream.topic_id,
                stream: streams::topic_events(stream.tenant_id, stream.topic_id),
            })
            .collect();
        stream_worker_master
            .reconcile(&stream_worker_assignement, &self.shutdown_token)
            .await;
        // 2. Reconcile delivery workers from active destinations in Redis
        let active_destinations: HashSet<(Uuid, Uuid)> = self
            .delivery_store
            .list_active_destinations()
            .await
            .context("Failed to list active destinations")?
            .into_iter()
            .collect();
        let delivery_assignements: HashSet<_> = active_destinations
            .iter()
            .map(|(tenant_id, destination_id)| DeliveryAssignment {
                tenant_id: *tenant_id,
                destination_id: *destination_id,
            })
            .collect();
        delivery_worker_master
            .reconcile(&delivery_assignements, &self.shutdown_token)
            .await;

        // 3. Reconcile retry workers (keep idle workers warm; only reap finished handles)
        let retry_worker_assignments: HashSet<RetryAssignment> = active_destinations
            .iter()
            .map(|(tenant_id, destination_id)| RetryAssignment {
                tenant_id: *tenant_id,
                destination_id: *destination_id,
            })
            .collect();
        retry_worker_master
            .reconcile(&retry_worker_assignments, &self.shutdown_token)
            .await;

        let span = tracing::Span::current();
        span.record("active_streams", stream_worker_master.worker_len());
        span.record("active_deliveries", delivery_worker_master.worker_len());
        span.record("active_retries", retry_worker_master.worker_len());
        Ok(())
    }
}
