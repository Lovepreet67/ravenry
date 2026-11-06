use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use iam_contracts::topic_registory::TopicRegistry;
use streamforge::traits::Queue;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    application::{
        delivery::service::DeliveryService,
        worker::{
            dlq_wokrer::DlqWorker, retry_worker::RetryWorker, stream_worker::StreamWorker,
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
        let mut stream_workers: HashMap<String, StreamWorker> = HashMap::new();
        let mut retry_workers: HashMap<(Uuid, Uuid), RetryWorker> = HashMap::new();
        let mut dlq_workers: HashMap<(Uuid, Uuid), DlqWorker> = HashMap::new();
        while !self.shutdown_token.is_cancelled() {
            if let Err(err) = self
                .reconcile_consumer(&mut stream_workers, &mut retry_workers, &mut dlq_workers)
                .await
            {
                warn!(error=%err, "Worker reconcilation failed");
            }
            sleep_or_cancel(&self.shutdown_token, reconcile_interval).await;
        }
    }

    async fn reconcile_consumer(
        self: &Arc<Self>,
        stream_workers: &mut HashMap<String, StreamWorker>,
        retry_workers: &mut HashMap<(Uuid, Uuid), RetryWorker>,
        dlq_workers: &mut HashMap<(Uuid, Uuid), DlqWorker>,
    ) -> Result<()> {
        let streams: Vec<_> = self
            .topic_registory
            .list_active()
            .await?
            .into_iter()
            .collect();
        let stream_key_set: HashSet<String> = streams.iter().map(|t| t.destination()).collect();
        // 1. reconcile the main worker
        let obsolute_stream_wokers: Vec<String> = stream_workers
            .iter()
            .filter(|(key, worker)| !stream_key_set.contains(*key) || worker.is_finished())
            .map(|(key, _)| key.clone())
            .collect();
        for key in obsolute_stream_wokers {
            if let Some(woker) = stream_workers.remove(&key) {
                woker.cancel().await;
            }
        }
        for stream in &streams {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let destination_key = stream.destination();
            if stream_workers.contains_key(&destination_key) {
                continue;
            }
            self.queue
                .ensure_group(
                    &destination_key,
                    &self.config.stream_worker_config.consumer_group,
                )
                .await?;
            let token = self.shutdown_token.child_token();
            let worker_stream = destination_key.clone();
            let id = Uuid::new_v4();
            let worker = StreamWorker::run(
                id,
                worker_stream,
                self.queue.clone(),
                self.delivery_service.clone(),
                self.config.stream_worker_config.clone(),
                token,
            );
            stream_workers.insert(destination_key, worker);
        }
        // 2. reconcile the retry worker
        let mut destination_key_set = HashSet::<(Uuid, Uuid)>::new();

        for topic in streams {
            let resolved = match self
                .projection_store
                .resolve_topic_destinations(topic.tenant_id, topic.topic_id)
                .await
            {
                Ok(destinations) => destinations,
                Err(error) => {
                    warn!(
                        tenant_id = %topic.tenant_id,
                        topic_id = %topic.topic_id,
                        error = %error,
                        "Failed to resolve destinations for retry discovery"
                    );
                    continue;
                }
            };

            for destination in resolved {
                if destination.config.status == "active" {
                    destination_key_set.insert((topic.tenant_id, destination.id));
                }
            }
        }
        let obsolute_retry_wokers: Vec<(Uuid, Uuid)> = retry_workers
            .iter()
            .filter(|(key, worker)| !destination_key_set.contains(*key) || worker.is_finished())
            .map(|(key, _)| key.clone())
            .collect();
        for key in obsolute_retry_wokers {
            if let Some(woker) = retry_workers.remove(&key) {
                woker.cancel().await;
            }
        }
        for worker_key in destination_key_set {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            if retry_workers.contains_key(&worker_key) {
                continue;
            }
            let token = self.shutdown_token.child_token();
            let id = Uuid::new_v4();
            let worker = RetryWorker::run(
                id,
                worker_key.0,
                worker_key.1,
                self.delivery_store.clone(),
                self.delivery_service.clone(),
                self.config.retry_worker_config.clone(),
                token,
            );
            retry_workers.insert(worker_key, worker);
        }
        // 3. reconcile the dlq worker
        let pending_dlq_destinations: HashSet<(Uuid, Uuid)> = self
            .delivery_store
            .list_pending_dlq_destinations()
            .await
            .context("Failed to list the pending dlq messages")?
            .iter()
            .map(|x| (x.tenant_id, x.destination_id))
            .collect();

        let obsolute_dlq_workers: Vec<_> = dlq_workers
            .iter()
            .filter(|(key, worker)| {
                worker.is_finished() || !pending_dlq_destinations.contains(*key)
            })
            .map(|(key, _)| key.clone())
            .collect();
        for odw in obsolute_dlq_workers {
            if let Some(worker) = dlq_workers.remove(&odw) {
                worker.cancel().await;
            }
        }
        for worker_key in pending_dlq_destinations {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            if dlq_workers.contains_key(&worker_key) {
                continue;
            }
            let token = self.shutdown_token.child_token();
            let id = Uuid::new_v4();
            let worker = DlqWorker::run(
                id,
                worker_key.0,
                worker_key.1,
                self.queue.clone(),
                self.config.dlq_worker_config.clone(),
                self.delivery_store.clone(),
                token,
            );
            dlq_workers.insert(worker_key, worker);
        }
        Ok(())
    }
}
