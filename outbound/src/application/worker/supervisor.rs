use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use iam_contracts::{streams, topic_registory::TopicRegistry};
use metrics::gauge;
use mimir::names::METRIC_ACTIVE_WORKERS;
use streamforge::traits::Queue;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::{
    application::{
        delivery::service::DeliveryService,
        worker::{
            delivery_worker::{DeliveryAssignment, DeliveryReport, DeliveryWorkerHandle},
            dlq_wokrer::DlqWorker,
            manual_retry_worker::ManualRetryWorker,
            retry_worker::{
                RetryAssignement, RetryReport, RetryWorker, RetryWorkerHandle, RetryWorkerMaster,
            },
            stream_worker::{StreamAssignment, StreamWorkerHandle, StreamWorkerReport},
            util::sleep_or_cancel,
        },
    },
    config::worker::WorkerRuntimeConfig,
    platform::{projection_store::ProjectionStore, redis::DeliveryStore},
};

const IDLE_PASS_TERMINATE_THRESHOLD: u32 = 3;
const STREAM_REPORT_CHANNEL_CAP: usize = 256;
const STREAM_WORKER_CHANNEL_CAP: usize = 64;
const DELIVERY_REPORT_CHANNEL_CAP: usize = 256;
const DELIVERY_WORKER_CHANNEL_CAP: usize = 64;
const RETRY_REPORT_CHANNEL_CAP: usize = 256;
const RETRY_WORKER_CHANNEL_CAP: usize = 64;

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

        let (stream_report_tx, mut stream_report_rx) =
            mpsc::channel::<StreamWorkerReport>(STREAM_REPORT_CHANNEL_CAP);
        let (delivery_report_tx, mut delivery_report_rx) =
            mpsc::channel::<DeliveryReport>(DELIVERY_REPORT_CHANNEL_CAP);
        let (retry_report_tx, retry_report_rx) =
            mpsc::channel::<RetryReport>(RETRY_REPORT_CHANNEL_CAP);

        let mut stream_workers: HashMap<Uuid, StreamWorkerHandle> = HashMap::new();
        let mut assigned_streams: HashSet<String> = HashSet::new();
        let mut worker_to_streams: HashMap<Uuid, HashSet<String>> = HashMap::new();

        let mut delivery_workers: HashMap<Uuid, DeliveryWorkerHandle> = HashMap::new();
        let mut assigned_destinations: HashSet<(Uuid, Uuid)> = HashSet::new();
        let mut worker_to_destinations: HashMap<Uuid, HashSet<(Uuid, Uuid)>> = HashMap::new();

        let mut retry_workers: HashMap<(Uuid, Uuid), RetryWorker> = HashMap::new();
        let mut manual_retry_workers: HashMap<Uuid, ManualRetryWorker> = HashMap::new();
        let mut dlq_workers: HashMap<(Uuid, Uuid), DlqWorker> = HashMap::new();

        let mut retry_worker_master = RetryWorkerMaster {
            report_tx: retry_report_tx,
            report_rx: retry_report_rx,
            workers: HashMap::new(),
            worker_to_destinations: HashMap::new(),
            destination_to_worker: HashMap::new(),
        };

        while !self.shutdown_token.is_cancelled() {
            if let Err(err) = self
                .reconcile_consumer(
                    &stream_report_tx,
                    &mut stream_report_rx,
                    &mut stream_workers,
                    &mut assigned_streams,
                    &mut worker_to_streams,
                    &delivery_report_tx,
                    &mut delivery_report_rx,
                    &mut delivery_workers,
                    &mut retry_worker_master,
                    &mut assigned_destinations,
                    &mut worker_to_destinations,
                    &mut dlq_workers,
                    &mut manual_retry_workers,
                )
                .await
            {
                warn!(error=%err, "Worker reconcilation failed");
            }
            self.record_worker_gauges(
                &stream_workers,
                &delivery_workers,
                &retry_workers,
                &dlq_workers,
                &manual_retry_workers,
            );
            sleep_or_cancel(&self.shutdown_token, reconcile_interval).await;
        }

        // Stop all stream and delivery workers cleanly on shutdown
        for (_, worker) in stream_workers.drain() {
            worker.cancel().await;
        }
        for (_, worker) in delivery_workers.drain() {
            worker.cancel().await;
        }
        assigned_streams.clear();
        worker_to_streams.clear();
        assigned_destinations.clear();
        worker_to_destinations.clear();
        retry_workers.clear();
        dlq_workers.clear();
        manual_retry_workers.clear();
        self.record_worker_gauges(
            &stream_workers,
            &delivery_workers,
            &retry_workers,
            &dlq_workers,
            &manual_retry_workers,
        );
    }

    fn record_worker_gauges(
        &self,
        stream_workers: &HashMap<Uuid, StreamWorkerHandle>,
        delivery_workers: &HashMap<Uuid, DeliveryWorkerHandle>,
        retry_workers: &HashMap<(Uuid, Uuid), RetryWorker>,
        dlq_workers: &HashMap<(Uuid, Uuid), DlqWorker>,
        manual_retry_workers: &HashMap<Uuid, ManualRetryWorker>,
    ) {
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "stream")
            .set(stream_workers.len() as f64);
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "delivery")
            .set(delivery_workers.len() as f64);
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "retry")
            .set(retry_workers.len() as f64);
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "dlq")
            .set(dlq_workers.len() as f64);
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "manual_retry")
            .set(manual_retry_workers.len() as f64);
    }

    #[instrument(
        skip(
            self,
            stream_report_tx,
            stream_report_rx,
            stream_workers,
            assigned_streams,
            worker_to_streams,
            delivery_report_tx,
            delivery_report_rx,
            delivery_workers,
            retry_worker_master,
            assigned_destinations,
            worker_to_destinations,
            dlq_workers,
            manaul_retry_workers,
        ),
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
        stream_report_tx: &mpsc::Sender<StreamWorkerReport>,
        stream_report_rx: &mut mpsc::Receiver<StreamWorkerReport>,
        stream_workers: &mut HashMap<Uuid, StreamWorkerHandle>,
        assigned_streams: &mut HashSet<String>,
        worker_to_streams: &mut HashMap<Uuid, HashSet<String>>,
        delivery_report_tx: &mpsc::Sender<DeliveryReport>,
        delivery_report_rx: &mut mpsc::Receiver<DeliveryReport>,
        delivery_workers: &mut HashMap<Uuid, DeliveryWorkerHandle>,
        retry_worker_master: &mut RetryWorkerMaster,
        assigned_destinations: &mut HashSet<(Uuid, Uuid)>,
        worker_to_destinations: &mut HashMap<Uuid, HashSet<(Uuid, Uuid)>>,
        dlq_workers: &mut HashMap<(Uuid, Uuid), DlqWorker>,
        manaul_retry_workers: &mut HashMap<Uuid, ManualRetryWorker>,
    ) -> Result<()> {
        let streams: Vec<_> = self
            .topic_registory
            .list_active()
            .await?
            .into_iter()
            .collect();
        let stream_key_set: HashSet<String> = streams.iter().map(|t| t.destination()).collect();

        // 1. Reconcile stream workers (reports, deprecations, and new assignments)
        let mut idle_candidates: HashSet<Uuid> = HashSet::new();
        while let Ok(report) = stream_report_rx.try_recv() {
            match report {
                StreamWorkerReport::Idle {
                    worker_id,
                    idle_passes,
                } => {
                    if idle_passes >= IDLE_PASS_TERMINATE_THRESHOLD {
                        idle_candidates.insert(worker_id);
                    }
                }
            }
        }

        // Remove finished or long-idle workers and unassign their streams
        let retired_workers: Vec<Uuid> = stream_workers
            .iter()
            .filter(|(id, worker)| worker.is_finished() || idle_candidates.contains(id))
            .map(|(id, _)| *id)
            .collect();
        for worker_id in retired_workers {
            if let Some(streams_for_worker) = worker_to_streams.remove(&worker_id) {
                for s in streams_for_worker {
                    assigned_streams.remove(&s);
                }
            }
            if let Some(worker) = stream_workers.remove(&worker_id) {
                debug!(worker_id = %worker_id, "Retiring finished or idle stream worker");
                worker.cancel().await;
            }
        }

        // Deprecate streams that are no longer in the active registry
        let removed_stream_keys: Vec<String> = assigned_streams
            .iter()
            .filter(|key| !stream_key_set.contains(*key))
            .cloned()
            .collect();
        for stream_key in removed_stream_keys {
            assigned_streams.remove(&stream_key);
            if let Some((owner_id, owner_streams)) = worker_to_streams
                .iter_mut()
                .find(|(_, set)| set.contains(&stream_key))
            {
                owner_streams.remove(&stream_key);
                if let Some(worker) = stream_workers.get(owner_id) {
                    let _ = worker.deprecate_tx.try_send(StreamAssignment {
                        tenant_id: Uuid::nil(),
                        topic_id: Uuid::nil(),
                        stream: stream_key.clone(),
                    });
                }
            }
        }

        // Assign new active streams across existing or newly spawned workers
        for stream in &streams {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            let destination_key = stream.destination();
            if assigned_streams.contains(&destination_key) {
                continue;
            }
            self.queue
                .ensure_group(
                    &destination_key,
                    &self.config.stream_worker_config.consumer_group,
                )
                .await?;

            // Pick a worker with the fewest assigned streams (or spawn a new one if none exist)
            let target_worker_id = if let Some((&existing_id, _)) =
                worker_to_streams.iter().min_by_key(|(_, set)| set.len())
            {
                existing_id
            } else {
                let id = Uuid::new_v4();
                let token = self.shutdown_token.child_token();
                let handle = StreamWorkerHandle::spawn(
                    id,
                    self.queue.clone(),
                    self.projection_store.clone(),
                    self.delivery_store.clone(),
                    self.config.stream_worker_config.clone(),
                    stream_report_tx.clone(),
                    STREAM_WORKER_CHANNEL_CAP,
                    token,
                );
                debug!(
                    worker_id = %id,
                    "Spawned stream worker"
                );
                stream_workers.insert(id, handle);
                worker_to_streams.insert(id, HashSet::new());
                id
            };

            let assignment = StreamAssignment {
                tenant_id: stream.tenant_id,
                topic_id: stream.topic_id,
                stream: destination_key.clone(),
            };

            if let Some(worker) = stream_workers.get(&target_worker_id) {
                if worker.add_tx.send(assignment).await.is_ok() {
                    assigned_streams.insert(destination_key.clone());
                    worker_to_streams
                        .entry(target_worker_id)
                        .or_default()
                        .insert(destination_key);
                } else {
                    warn!(
                        worker_id = %target_worker_id,
                        stream = %destination_key,
                        "Failed to send stream assignment; worker channel closed"
                    );
                }
            }
        }

        // 2. Reconcile delivery workers from active destinations in Redis
        let active_destinations: HashSet<(Uuid, Uuid)> = self
            .delivery_store
            .list_active_destinations()
            .await
            .context("Failed to list active destinations")?
            .into_iter()
            .collect();

        let mut idle_delivery_candidates: HashSet<Uuid> = HashSet::new();
        while let Ok(report) = delivery_report_rx.try_recv() {
            match report {
                DeliveryReport::Idle {
                    worker_id,
                    idle_passes,
                } => {
                    if idle_passes >= IDLE_PASS_TERMINATE_THRESHOLD {
                        idle_delivery_candidates.insert(worker_id);
                    }
                }
            }
        }

        let retired_delivery_workers: Vec<Uuid> = delivery_workers
            .iter()
            .filter(|(id, worker)| worker.is_finished() || idle_delivery_candidates.contains(id))
            .map(|(id, _)| *id)
            .collect();
        for worker_id in retired_delivery_workers {
            if let Some(dests_for_worker) = worker_to_destinations.remove(&worker_id) {
                for dest in dests_for_worker {
                    assigned_destinations.remove(&dest);
                }
            }
            if let Some(worker) = delivery_workers.remove(&worker_id) {
                debug!(worker_id = %worker_id, "Retiring finished or idle delivery worker");
                worker.cancel().await;
            }
        }

        let removed_destinations: Vec<(Uuid, Uuid)> = assigned_destinations
            .iter()
            .filter(|dest| !active_destinations.contains(*dest))
            .copied()
            .collect();
        for dest in removed_destinations {
            assigned_destinations.remove(&dest);
            if let Some((owner_id, owner_dests)) = worker_to_destinations
                .iter_mut()
                .find(|(_, set)| set.contains(&dest))
            {
                owner_dests.remove(&dest);
                if let Some(worker) = delivery_workers.get(owner_id) {
                    let _ = worker.deprecate_tx.try_send(DeliveryAssignment {
                        tenant_id: dest.0,
                        destination_id: dest.1,
                    });
                }
            }
        }

        for dest in &active_destinations {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            if assigned_destinations.contains(dest) {
                continue;
            }

            let delivery_stream = streams::destination_deliveries(dest.0, dest.1);
            self.queue
                .ensure_group(
                    &delivery_stream,
                    &self.config.delivery_worker_config.consumer_group,
                )
                .await?;

            let target_worker_id = if let Some((&existing_id, _)) = worker_to_destinations
                .iter()
                .min_by_key(|(_, set)| set.len())
            {
                existing_id
            } else {
                let id = Uuid::new_v4();
                let token = self.shutdown_token.child_token();
                let handle = DeliveryWorkerHandle::spawn(
                    id,
                    self.queue.clone(),
                    self.delivery_service.clone(),
                    self.delivery_store.clone(),
                    self.config.delivery_worker_config.clone(),
                    delivery_report_tx.clone(),
                    DELIVERY_WORKER_CHANNEL_CAP,
                    token,
                );
                debug!(
                    worker_id = %id,
                    "Spawned delivery worker"
                );
                delivery_workers.insert(id, handle);
                worker_to_destinations.insert(id, HashSet::new());
                id
            };

            let assignment = DeliveryAssignment {
                tenant_id: dest.0,
                destination_id: dest.1,
            };

            if let Some(worker) = delivery_workers.get(&target_worker_id) {
                if worker.add_tx.send(assignment).await.is_ok() {
                    assigned_destinations.insert(*dest);
                    worker_to_destinations
                        .entry(target_worker_id)
                        .or_default()
                        .insert(*dest);
                } else {
                    warn!(
                        worker_id = %target_worker_id,
                        tenant_id = %dest.0,
                        destination_id = %dest.1,
                        "Failed to send delivery assignment; worker channel closed"
                    );
                }
            }
        }

        // 3. Reconcile retry workers (keep idle workers warm; only reap finished handles)
        while let Ok(report) = retry_worker_master.report_rx.try_recv() {
            match report {
                RetryReport::Idle {
                    worker_id,
                    idle_passes,
                } => {
                    tracing::debug!(
                        worker_id = %worker_id,
                        idle_passes = idle_passes,
                        "Retry worker idle"
                    );
                }
            }
        }

        let retired_retry_workers: Vec<Uuid> = retry_worker_master
            .workers
            .iter()
            .filter(|(_, worker)| worker.is_finished())
            .map(|(id, _)| *id)
            .collect();
        for worker_id in retired_retry_workers {
            if let Some(dests_for_worker) = retry_worker_master
                .worker_to_destinations
                .remove(&worker_id)
            {
                for dest in dests_for_worker {
                    retry_worker_master.destination_to_worker.remove(&dest);
                }
            }
            if let Some(worker) = retry_worker_master.workers.remove(&worker_id) {
                debug!(worker_id = %worker_id, "Retiring finished retry worker");
                worker.cancel().await;
            }
        }

        let removed_destinations: Vec<(Uuid, Uuid)> = retry_worker_master
            .destination_to_worker
            .keys()
            .filter(|dest| !active_destinations.contains(*dest))
            .copied()
            .collect();
        for dest in removed_destinations {
            if let Some(worker_id) = retry_worker_master.destination_to_worker.remove(&dest) {
                if let Some(destinations) = retry_worker_master
                    .worker_to_destinations
                    .get_mut(&worker_id)
                {
                    destinations.remove(&dest);
                }
                if let Some(worker_handle) = retry_worker_master.workers.get(&worker_id) {
                    if worker_handle
                        .deprecate_tx
                        .send(super::retry_worker::RetryAssignement {
                            tenant_id: dest.0,
                            destination_id: dest.1,
                        })
                        .await
                        .is_err()
                    {
                        warn!(
                            worker_id = %worker_id,
                            tenant_id = %dest.0,
                            destination_id = %dest.1,
                            "Failed to send retry deprecation; worker channel closed"
                        );
                    }
                }
            }
        }

        for dest in active_destinations {
            if self.shutdown_token.is_cancelled() {
                break;
            }
            if retry_worker_master
                .destination_to_worker
                .contains_key(&dest)
            {
                continue;
            }

            let target_worker_id = if let Some((&existing_id, _)) = retry_worker_master
                .worker_to_destinations
                .iter()
                .min_by_key(|(_, set)| set.len())
            {
                existing_id
            } else {
                let id = Uuid::new_v4();
                let token = self.shutdown_token.child_token();
                let handle = RetryWorkerHandle::spawn(
                    id,
                    self.delivery_store.clone(),
                    retry_worker_master.report_tx.clone(),
                    RETRY_WORKER_CHANNEL_CAP,
                    token,
                    self.config.retry_worker_config.clone(),
                );
                debug!(worker_id = %id, "Spawned retry worker");
                retry_worker_master.workers.insert(id, handle);
                retry_worker_master
                    .worker_to_destinations
                    .insert(id, HashSet::new());
                id
            };

            let assignment = RetryAssignement {
                tenant_id: dest.0,
                destination_id: dest.1,
            };

            if let Some(worker_handle) = retry_worker_master.workers.get(&target_worker_id) {
                if worker_handle.add_tx.send(assignment).await.is_ok() {
                    retry_worker_master
                        .destination_to_worker
                        .insert(dest, target_worker_id);
                    retry_worker_master
                        .worker_to_destinations
                        .entry(target_worker_id)
                        .or_default()
                        .insert(dest);
                } else {
                    warn!(
                        worker_id = %target_worker_id,
                        tenant_id = %dest.0,
                        destination_id = %dest.1,
                        "Failed to send retry assignment; worker channel closed"
                    );
                }
            }
        }

        // 4. reconcile the dlq worker
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
                debug!(
                    tenant_id = %odw.0,
                    destination_id = %odw.1,
                    "Stopping obsolete or finished dlq worker"
                );
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
            debug!(
                worker_id = %id,
                tenant_id = %worker_key.0,
                destination_id = %worker_key.1,
                "Spawned dlq worker"
            );
            dlq_workers.insert(worker_key, worker);
        }

        // 5. reconcile the manual retry worker
        self.queue
            .ensure_group(
                iam_contracts::streams::MANUALY_RETRY,
                &self.config.manual_retry_worker_config.consumer_group,
            )
            .await?;
        let obsolute_mr_workers: Vec<_> = manaul_retry_workers
            .iter()
            .filter(|(_, worker)| worker.is_finished())
            .map(|(key, _)| key.clone())
            .collect();
        for omr in obsolute_mr_workers {
            manaul_retry_workers.remove(&omr);
        }
        for _ in
            0..(self.config.manual_retry_worker_config.worker_count - manaul_retry_workers.len())
        {
            let stream = iam_contracts::streams::MANUALY_RETRY.to_string();
            let token = self.shutdown_token.clone();
            let id = Uuid::new_v4();
            let worker = ManualRetryWorker::run(
                id,
                stream,
                self.queue.clone(),
                self.delivery_store.clone(),
                self.delivery_service.clone(),
                self.config.manual_retry_worker_config.clone(),
                token,
            );
            manaul_retry_workers.insert(id, worker);
        }
        let span = tracing::Span::current();
        span.record("active_streams", stream_workers.len());
        span.record("active_deliveries", delivery_workers.len());
        span.record("active_retries", retry_worker_master.workers.len());
        span.record("active_dlqs", dlq_workers.len());
        span.record("active_manual_retries", manaul_retry_workers.len());
        Ok(())
    }
}
