use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use iam_contracts::{keys, streams};
use metrics::gauge;
use mimir::names::METRIC_ACTIVE_WORKERS;
use streamforge::{traits::Queue, Message};
use tokio::{
    sync::mpsc::{self, error::TrySendError},
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn, Instrument, Span};
use uuid::Uuid;

use crate::{
    application::{
        delivery::{model::DeliveryRequest, service::DeliveryService},
        worker::runner::{WorkerChannels, WorkerCommand, WorkerRunner, WorkerTask},
    },
    config::worker::DeliveryWorkerConfig,
    platform::redis::DeliveryStore,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeliveryAssignment {
    pub tenant_id: Uuid,
    pub destination_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum DeliveryReport {
    Idle { worker_id: Uuid, idle_passes: u32 },
}

/// Everything a delivery worker needs, bundled so the master can spawn workers.
#[derive(Clone)]
pub struct DeliveryWorkerDeps {
    pub queue: Arc<dyn Queue>,
    pub delivery_service: Arc<DeliveryService>,
    pub delivery_store: Arc<DeliveryStore>,
    pub config: Arc<DeliveryWorkerConfig>,
}

pub struct DeliveryWorker {
    id: Uuid,
    consumer_name: String,
    queue: Arc<dyn Queue>,
    delivery_service: Arc<DeliveryService>,
    delivery_store: Arc<DeliveryStore>,
    config: Arc<DeliveryWorkerConfig>,
    token: CancellationToken,
}

pub struct DeliveryWorkerHandle {
    pub id: Uuid,
    cmd_tx: mpsc::Sender<WorkerCommand<DeliveryAssignment>>,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl DeliveryWorkerHandle {
    pub fn spawn(
        id: Uuid,
        deps: &DeliveryWorkerDeps,
        report_tx: mpsc::Sender<DeliveryReport>,
        channel_cap: usize,
        token: CancellationToken,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(channel_cap);
        let idle_sleep = Duration::from_secs(deps.config.idle_interval_secs);
        let consumer_name = format!("{}_{}", deps.config.consumer_name_prefix, id);
        let group = deps.config.consumer_group.clone();

        let worker = DeliveryWorker {
            id,
            consumer_name: consumer_name.clone(),
            queue: deps.queue.clone(),
            delivery_service: deps.delivery_service.clone(),
            delivery_store: deps.delivery_store.clone(),
            config: deps.config.clone(),
            token: token.clone(),
        };

        let channels = WorkerChannels { cmd_rx, report_tx };
        let task_token = token.clone();

        let handle = tokio::spawn(async move {
            info!(
                worker_id = %id,
                group = %group,
                consumer = %consumer_name,
                "Delivery worker started"
            );

            WorkerRunner::new(worker, channels, idle_sleep, task_token)
                .run()
                .await;

            info!(
                worker_id = %id,
                group = %group,
                consumer = %consumer_name,
                "Delivery worker stopped"
            );
        });

        Self {
            id,
            cmd_tx,
            token,
            handle,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// Awaits a worker that has already finished and logs a panic/cancel error.
    pub async fn reap(self) {
        if let Err(err) = self.handle.await {
            warn!(
                worker_id = %self.id,
                error = %err,
                "Delivery worker join failed"
            );
        }
    }

    /// Signals cancellation, waits up to `timeout`, then aborts the task.
    pub async fn shutdown(self, timeout: Duration) {
        self.token.cancel();

        let id = self.id;
        let mut handle = self.handle;

        match tokio::time::timeout(timeout, &mut handle).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                warn!(
                    worker_id = %id,
                    error = %err,
                    "Delivery worker join failed"
                );
            }
            Err(_) => {
                warn!(
                    worker_id = %id,
                    "Delivery worker did not stop in time; aborting"
                );
                handle.abort();
            }
        }
    }
}

#[async_trait]
impl WorkerTask for DeliveryWorker {
    type Assignment = DeliveryAssignment;
    type Report = DeliveryReport;

    async fn step(&mut self, assignment: &Self::Assignment) -> Result<bool, anyhow::Error> {
        let stream =
            streams::destination_deliveries(assignment.tenant_id, assignment.destination_id);
        let batch_span = tracing::info_span!(
            "delivery.process_batch",
            worker_id = %self.id,
            tenant_id = %assignment.tenant_id,
            destination_id = %assignment.destination_id,
            stream = %stream,
            consumer = %self.consumer_name,
            acked = tracing::field::Empty,
            batch_size = tracing::field::Empty,
        );

        let queue = self.queue.clone();
        let delivery_service = self.delivery_service.clone();
        let delivery_store = self.delivery_store.clone();
        let config = self.config.clone();
        let token = self.token.clone();
        let consumer_name = self.consumer_name.clone();

        async move {
            // ensure that group exist
            if let Err(err) = queue
                .ensure_group(&stream, config.consumer_group.as_str())
                .await
            {
                error!(err=%err, stream,"Error while creating group");
                return Ok(false);
            };
            let deliveries = queue
                .consume(
                    &stream,
                    &config.consumer_group,
                    consumer_name.as_str(),
                    config.batch_size,
                    1000,
                )
                .await
                .map_err(|err| anyhow::anyhow!("consume failed: {err}"))?;

            Span::current().record("batch_size", deliveries.len());

            if deliveries.is_empty() {
                return Ok(false);
            }

            let mut join_set = JoinSet::new();
            let mut invalid_deliveries = Vec::new();
            let mut invalid_delivery_ids = Vec::new();
            let mut ack_ids = Vec::with_capacity(deliveries.len());
            let result_stream = Arc::new(streams::destination_outcomes(
                assignment.tenant_id,
                assignment.destination_id,
            ));
            let retry_set_key = Arc::new(keys::destination_retry(
                assignment.tenant_id,
                assignment.destination_id,
            ));

            for delivery in deliveries {
                // Unstarted deliveries remain pending if shutdown is requested.
                // Tasks already started are joined below before their IDs are ACKed.
                if token.is_cancelled() {
                    break;
                }

                let request: DeliveryRequest =
                    match serde_json::from_slice(&delivery.message.payload) {
                        Ok(request) => request,
                        Err(err) => {
                            warn!(
                                queue_message_id = %delivery.delivery_id,
                                error = %err,
                                "Invalid delivery request"
                            );
                            invalid_deliveries.push(delivery.message);
                            invalid_delivery_ids.push(delivery.delivery_id);
                            continue;
                        }
                    };

                let delivery_id = delivery.delivery_id.clone();
                let delivery_service = delivery_service.clone();
                let delivery_store = delivery_store.clone();
                let queue = queue.clone();
                let result_stream = result_stream.clone();
                let retry_set_key = retry_set_key.clone();

                join_set.spawn(async move {
                    let outcome = delivery_service
                        .execute_delivery(request.clone())
                        .await
                        .map_err(|err| anyhow::anyhow!("execute_delivery failed: {err}"))?;

                    let payload = serde_json::to_vec(&outcome)
                        .map_err(|err| anyhow::anyhow!("serialize outcome failed: {err}"))?;

                    queue
                        .publish(&result_stream, Message::new(payload))
                        .await
                        .map_err(|err| anyhow::anyhow!("publish outcome failed: {err}"))?;

                    if outcome.will_retry {
                        let retry_at = outcome
                            .next_attempt_at
                            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::seconds(5));
                        let score_ms = retry_at.timestamp_millis();

                        let mut retry_request = request;
                        retry_request.curr_attempt = Some(outcome.attempt + 1);

                        delivery_store
                            .schedule_retry(&retry_set_key, &retry_request, score_ms)
                            .await
                            .map_err(|err| anyhow::anyhow!("schedule_retry failed: {err}"))?;
                    }

                    Ok::<String, anyhow::Error>(delivery_id)
                });
            }

            // Invalid payloads are ACKed only after they have been copied to
            // the destination DLQ successfully.
            if !invalid_deliveries.is_empty() {
                let dlq_key =
                    streams::destination_dlq(assignment.tenant_id, assignment.destination_id);

                queue
                    .publish_batch(
                        invalid_deliveries
                            .into_iter()
                            .map(|message| (dlq_key.as_str(), message))
                            .collect(),
                    )
                    .await
                    .map_err(|err| {
                        anyhow::anyhow!("failed to publish invalid delivery requests to DLQ: {err}")
                    })?;

                ack_ids.extend(invalid_delivery_ids);
            }

            let mut tasks_failed = 0usize;

            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(Ok(delivery_id)) => ack_ids.push(delivery_id),
                    Ok(Err(err)) => {
                        tasks_failed += 1;
                        warn!(
                            error = %err,
                            "Delivery task failed; leaving its message pending"
                        );
                    }
                    Err(join_err) => {
                        tasks_failed += 1;
                        warn!(
                            error = %join_err,
                            "Delivery task panicked or was cancelled; leaving its message pending"
                        );
                    }
                }
            }

            if tasks_failed > 0 {
                warn!(
                    failed_tasks = tasks_failed,
                    acked_count = ack_ids.len(),
                    "Some delivery messages will remain pending"
                );
            }

            if !ack_ids.is_empty() {
                let ack_refs: Vec<&str> = ack_ids.iter().map(String::as_str).collect();

                queue
                    .ack_batch(&stream, &config.consumer_group, ack_refs.as_slice())
                    .await
                    .map_err(|err| anyhow::anyhow!("delivery batch ACK failed: {err}"))?;

                Span::current().record("acked", true);
                debug!(acked_count = ack_refs.len(), "Delivery batch ACKed");
            } else {
                Span::current().record("acked", false);
            }

            Ok(true)
        }
        .instrument(batch_span)
        .await
    }

    fn on_idle(&mut self, idle_passes: u32) -> Option<Self::Report> {
        Some(DeliveryReport::Idle {
            worker_id: self.id,
            idle_passes,
        })
    }
}

const DELIVERY_WORKER_CHANNEL_CAP: usize = 250;
const DELIVERY_REPORT_CHANNEL_CAP: usize = 250;

/// Hard ceiling on the number of delivery workers.
const MAX_WORKERS: usize = 10;

/// Destination assignments per worker before another worker is spawned
/// (until `MAX_WORKERS` is reached).
const DESTINATIONS_PER_WORKER_SOFT_CAP: usize = 10;

struct WorkerSlot {
    handle: DeliveryWorkerHandle,
    assignments: HashSet<DeliveryAssignment>,
}

pub struct DeliveryWorkerMaster {
    report_rx: mpsc::Receiver<DeliveryReport>,
    report_tx: mpsc::Sender<DeliveryReport>,
    workers: HashMap<Uuid, WorkerSlot>,
    assignment_to_worker: HashMap<DeliveryAssignment, Uuid>,
    deps: DeliveryWorkerDeps,
}

/// Pure placement decision. `live` contains (worker_id, current_load) for live
/// workers only. Returns a worker to use, or `None` if a new one should spawn.
fn choose_worker(live: &[(Uuid, usize)]) -> Option<Uuid> {
    let &(id, load) = live.iter().min_by_key(|(_, load)| *load)?;

    (load < DESTINATIONS_PER_WORKER_SOFT_CAP || live.len() >= MAX_WORKERS).then_some(id)
}

impl DeliveryWorkerMaster {
    pub fn new(deps: DeliveryWorkerDeps) -> Self {
        let (report_tx, report_rx) = mpsc::channel(DELIVERY_REPORT_CHANNEL_CAP);

        Self {
            report_tx,
            report_rx,
            workers: HashMap::new(),
            assignment_to_worker: HashMap::new(),
            deps,
        }
    }
    pub fn worker_len(&self) -> usize {
        self.workers.len()
    }
    fn drain_reports(&mut self) {
        while let Ok(report) = self.report_rx.try_recv() {
            match report {
                DeliveryReport::Idle {
                    worker_id,
                    idle_passes,
                } => {
                    debug!(
                        worker_id = %worker_id,
                        idle_passes,
                        "Delivery worker idle"
                    );
                }
            }
        }
    }

    fn spawn_worker(&mut self, shutdown_token: &CancellationToken) -> Uuid {
        let id = Uuid::new_v4();
        let handle = DeliveryWorkerHandle::spawn(
            id,
            &self.deps,
            self.report_tx.clone(),
            DELIVERY_WORKER_CHANNEL_CAP,
            shutdown_token.child_token(),
        );

        debug!(worker_id = %id, "Spawned delivery worker");

        self.workers.insert(
            id,
            WorkerSlot {
                handle,
                assignments: HashSet::new(),
            },
        );

        id
    }

    /// Runs one reconciliation pass. Level-triggered and idempotent: work that
    /// cannot be completed in this pass is retried during the next pass.
    pub async fn reconcile(
        &mut self,
        active: &HashSet<DeliveryAssignment>,
        shutdown_token: &CancellationToken,
    ) {
        self.drain_reports();
        self.reap_finished().await;
        self.remove_inactive(active);
        self.assign_unassigned(active, shutdown_token).await;

        let live = self
            .workers
            .values()
            .filter(|slot| !slot.handle.is_finished())
            .count();

        gauge!(
            METRIC_ACTIVE_WORKERS,
            mimir::lables::WORKER_TYPE => "delivery"
        )
        .set(live as f64);
    }

    /// Cancels every worker and waits for each one, bounded by `timeout`.
    pub async fn shutdown(&mut self, timeout: Duration) {
        let slots: Vec<WorkerSlot> = self.workers.drain().map(|(_, slot)| slot).collect();

        self.assignment_to_worker.clear();

        // Cancel all workers first so shutdown happens concurrently.
        for slot in &slots {
            slot.handle.token.cancel();
        }

        for slot in slots {
            slot.handle.shutdown(timeout).await;
        }

        gauge!(
            METRIC_ACTIVE_WORKERS,
            mimir::lables::WORKER_TYPE => "delivery"
        )
        .set(0.0);
    }

    async fn reap_finished(&mut self) {
        let finished: Vec<Uuid> = self
            .workers
            .iter()
            .filter(|(_, slot)| slot.handle.is_finished())
            .map(|(id, _)| *id)
            .collect();

        for worker_id in finished {
            if let Some(slot) = self.workers.remove(&worker_id) {
                for assignment in &slot.assignments {
                    self.assignment_to_worker.remove(assignment);
                }

                debug!(
                    worker_id = %worker_id,
                    "Retiring finished delivery worker"
                );

                slot.handle.reap().await;
            }
        }
    }

    /// Sends Remove for assignments that are no longer active. Internal
    /// bookkeeping changes only after the command is successfully queued.
    fn remove_inactive(&mut self, active: &HashSet<DeliveryAssignment>) {
        let stale: Vec<DeliveryAssignment> = self
            .assignment_to_worker
            .keys()
            .filter(|assignment| !active.contains(*assignment))
            .cloned()
            .collect();

        for assignment in stale {
            let Some(&worker_id) = self.assignment_to_worker.get(&assignment) else {
                continue;
            };

            let Some(slot) = self.workers.get_mut(&worker_id) else {
                self.assignment_to_worker.remove(&assignment);
                continue;
            };

            match slot
                .handle
                .cmd_tx
                .try_send(WorkerCommand::Remove(assignment.clone()))
            {
                Ok(()) => {
                    slot.assignments.remove(&assignment);
                    self.assignment_to_worker.remove(&assignment);
                }
                Err(TrySendError::Full(_)) => {
                    debug!(
                        worker_id = %worker_id,
                        tenant_id = %assignment.tenant_id,
                        destination_id = %assignment.destination_id,
                        "Delivery worker channel full; deferring removal"
                    );
                }
                Err(TrySendError::Closed(_)) => {
                    // The next reconciliation pass reaps the finished worker.
                    debug!(
                        worker_id = %worker_id,
                        "Delivery worker channel closed during removal"
                    );
                }
            }
        }
    }

    async fn assign_unassigned(
        &mut self,
        active: &HashSet<DeliveryAssignment>,
        shutdown_token: &CancellationToken,
    ) {
        for assignment in active {
            if shutdown_token.is_cancelled() {
                break;
            }

            if self.assignment_to_worker.contains_key(assignment) {
                continue;
            }

            let live: Vec<(Uuid, usize)> = self
                .workers
                .iter()
                .filter(|(_, slot)| !slot.handle.is_finished())
                .map(|(id, slot)| (*id, slot.assignments.len()))
                .collect();

            let worker_id = match choose_worker(&live) {
                Some(id) => id,
                None => self.spawn_worker(shutdown_token),
            };

            let Some(slot) = self.workers.get_mut(&worker_id) else {
                continue;
            };

            match slot
                .handle
                .cmd_tx
                .try_send(WorkerCommand::Add(assignment.clone()))
            {
                Ok(()) => {
                    slot.assignments.insert(assignment.clone());
                    self.assignment_to_worker
                        .insert(assignment.clone(), worker_id);
                }
                Err(TrySendError::Full(_)) => {
                    debug!(
                        worker_id = %worker_id,
                        tenant_id = %assignment.tenant_id,
                        destination_id = %assignment.destination_id,
                        "Delivery worker channel full; assignment deferred to next pass"
                    );
                }
                Err(TrySendError::Closed(_)) => {
                    warn!(
                        worker_id = %worker_id,
                        tenant_id = %assignment.tenant_id,
                        destination_id = %assignment.destination_id,
                        "Failed to send delivery assignment; worker channel closed"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker_with_load(load: usize) -> (Uuid, usize) {
        (Uuid::new_v4(), load)
    }

    #[test]
    fn spawns_when_no_workers_are_live() {
        assert_eq!(choose_worker(&[]), None);
    }

    #[test]
    fn picks_the_least_loaded_worker_under_the_soft_cap() {
        let a = worker_with_load(3);
        let b = worker_with_load(1);

        assert_eq!(choose_worker(&[a, b]), Some(b.0));
    }

    #[test]
    fn spawns_when_all_workers_are_full_and_below_maximum() {
        let live = vec![worker_with_load(DESTINATIONS_PER_WORKER_SOFT_CAP); 3];

        assert_eq!(choose_worker(&live), None);
    }

    #[test]
    fn reuses_a_worker_when_all_workers_are_full_at_maximum() {
        let live = vec![worker_with_load(DESTINATIONS_PER_WORKER_SOFT_CAP + 5); MAX_WORKERS];

        assert!(choose_worker(&live).is_some());
    }
}
