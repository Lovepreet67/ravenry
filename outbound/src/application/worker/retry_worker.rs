use async_trait::async_trait;
use iam_contracts::{keys, streams};
use metrics::gauge;
use mimir::names::METRIC_ACTIVE_WORKERS;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    application::worker::runner::{WorkerChannels, WorkerCommand, WorkerRunner, WorkerTask},
    config::worker::RetryWorkerConfig,
    platform::redis::DeliveryStore,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RetryAssignment {
    pub tenant_id: Uuid,
    pub destination_id: Uuid,
}

pub enum RetryReport {
    Idle { worker_id: Uuid, idle_passes: u32 },
}

pub struct RetryWorker {
    id: Uuid,
    delivery_store: Arc<DeliveryStore>,
    config: Arc<RetryWorkerConfig>,
}

pub struct RetryWorkerHandle {
    pub id: Uuid,
    cmd_tx: mpsc::Sender<WorkerCommand<RetryAssignment>>,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl RetryWorkerHandle {
    pub fn spawn(
        id: Uuid,
        delivery_store: Arc<DeliveryStore>,
        report_tx: mpsc::Sender<RetryReport>,
        channel_cap: usize,
        token: CancellationToken,
        config: Arc<RetryWorkerConfig>,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(channel_cap);
        let idle_sleep = Duration::from_millis(config.idle_interval_millisecs);
        let worker = RetryWorker {
            id,
            delivery_store,
            config,
        };
        let channels = WorkerChannels { cmd_rx, report_tx };
        let task_token = token.clone();
        let handle = tokio::spawn(async move {
            info!(worker_id = %id, "Retry Worker started");
            WorkerRunner::new(worker, channels, idle_sleep, task_token)
                .run()
                .await;
            info!(worker_id = %id, "Retry Worker stopped");
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
                "Retry worker join failed"
            );
        }
    }

    /// Signals cancellation, waits up to `timeout`, then aborts.
    pub async fn shutdown(self, timeout: Duration) {
        self.token.cancel();
        let id = self.id;
        let mut handle = self.handle;
        match tokio::time::timeout(timeout, &mut handle).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                warn!(worker_id = %id, error = %err, "Retry worker join failed")
            }
            Err(_) => {
                warn!(worker_id = %id, "Retry worker did not stop in time; aborting");
                handle.abort();
            }
        }
    }
}

#[async_trait]
impl WorkerTask for RetryWorker {
    type Assignment = RetryAssignment;
    type Report = RetryReport;

    async fn step(&mut self, assignment: &Self::Assignment) -> Result<bool, anyhow::Error> {
        let retry_key = keys::destination_retry(assignment.tenant_id, assignment.destination_id);
        let delivery_stream_key =
            streams::destination_deliveries(assignment.tenant_id, assignment.destination_id);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let moved = self
            .delivery_store
            .promote_due_retries(
                &retry_key,
                &delivery_stream_key,
                now_ms,
                self.config.batch_size,
            )
            .await?;
        Ok(moved > 0)
    }

    fn on_idle(&mut self, idle_passes: u32) -> Option<Self::Report> {
        Some(RetryReport::Idle {
            worker_id: self.id,
            idle_passes,
        })
    }
}

const RETRY_WORKER_CHANNEL_CAP: usize = 250;
const RETRY_REPORT_CHANNEL_CAP: usize = 250;
const MAX_WORKERS: usize = 10;
const DESTS_PER_WORKER_SOFT_CAP: usize = 10;

struct WorkerSlot {
    handle: RetryWorkerHandle,
    assignments: HashSet<RetryAssignment>,
}

pub struct RetryWorkerMaster {
    report_rx: mpsc::Receiver<RetryReport>,
    report_tx: mpsc::Sender<RetryReport>,
    workers: HashMap<Uuid, WorkerSlot>,
    assignment_to_worker: HashMap<RetryAssignment, Uuid>,
    delivery_store: Arc<DeliveryStore>,
    config: Arc<RetryWorkerConfig>,
}

/// Pure placement decision. `live` is (worker_id, current_load) for live workers
/// only. Returns the worker to use, or `None` if a new one should be spawned.
fn choose_worker(live: &[(Uuid, usize)]) -> Option<Uuid> {
    let &(id, load) = live.iter().min_by_key(|(_, load)| *load)?;
    (load < DESTS_PER_WORKER_SOFT_CAP || live.len() >= MAX_WORKERS).then_some(id)
}

impl RetryWorkerMaster {
    pub fn new(delivery_store: Arc<DeliveryStore>, config: Arc<RetryWorkerConfig>) -> Self {
        let (report_tx, report_rx) = mpsc::channel::<RetryReport>(RETRY_REPORT_CHANNEL_CAP);
        Self {
            report_tx,
            report_rx,
            workers: HashMap::new(),
            assignment_to_worker: HashMap::new(),
            delivery_store,
            config,
        }
    }
    pub fn worker_len(&self) -> usize {
        self.workers.len()
    }
    fn drain_reports(&mut self) {
        while let Ok(report) = self.report_rx.try_recv() {
            match report {
                RetryReport::Idle {
                    worker_id,
                    idle_passes,
                } => {
                    debug!(
                        worker_id = %worker_id,
                        idle_passes = idle_passes,
                        "Retry worker idle"
                    );
                }
            }
        }
    }

    fn spawn_worker(&mut self, shutdown_token: &CancellationToken) -> Uuid {
        let id = Uuid::new_v4();
        let handle = RetryWorkerHandle::spawn(
            id,
            self.delivery_store.clone(),
            self.report_tx.clone(),
            RETRY_WORKER_CHANNEL_CAP,
            shutdown_token.child_token(),
            self.config.clone(),
        );
        debug!(worker_id = %id, "Spawned retry worker");
        self.workers.insert(
            id,
            WorkerSlot {
                handle,
                assignments: HashSet::new(),
            },
        );
        id
    }

    /// Runs one reconciliation pass. Level-triggered and idempotent: anything
    /// that can't be done this pass (full channel, dead worker) is retried on
    /// the next one.
    pub async fn reconcile(
        &mut self,
        active: &HashSet<RetryAssignment>,
        shutdown_token: &CancellationToken,
    ) {
        self.drain_reports();
        self.reap_finished().await;
        self.remove_inactive(active);
        self.assign_unassigned(active, shutdown_token);

        let live = self
            .workers
            .values()
            .filter(|slot| !slot.handle.is_finished())
            .count();
        // NOTE: `lables` is misspelled in the mimir crate; rename it there.
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "retry").set(live as f64);
    }

    /// Cancels every worker and waits for them (bounded by `timeout` each).
    pub async fn shutdown(&mut self, timeout: Duration) {
        let slots: Vec<WorkerSlot> = self.workers.drain().map(|(_, slot)| slot).collect();
        self.assignment_to_worker.clear();
        for slot in &slots {
            slot.handle.token.cancel();
        }
        for slot in slots {
            slot.handle.shutdown(timeout).await;
        }
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
                debug!(worker_id = %worker_id, "Retiring finished retry worker");
                slot.handle.reap().await;
            }
        }
    }

    /// Sends `Remove` for assignments that are no longer active. Maps are only
    /// updated after a successful send, so a full channel just defers removal.
    fn remove_inactive(&mut self, active: &HashSet<RetryAssignment>) {
        let stale: Vec<RetryAssignment> = self
            .assignment_to_worker
            .keys()
            .filter(|a| !active.contains(*a))
            .copied()
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
                .try_send(WorkerCommand::Remove(assignment))
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
                        "Retry worker channel full; deferring removal"
                    );
                }
                Err(TrySendError::Closed(_)) => {
                    // Worker is gone; reap_finished clears its state next pass.
                    debug!(worker_id = %worker_id, "Retry worker channel closed during removal");
                }
            }
        }
    }

    fn assign_unassigned(
        &mut self,
        active: &HashSet<RetryAssignment>,
        shutdown_token: &CancellationToken,
    ) {
        for &assignment in active {
            if shutdown_token.is_cancelled() {
                break;
            }
            if self.assignment_to_worker.contains_key(&assignment) {
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
            match slot.handle.cmd_tx.try_send(WorkerCommand::Add(assignment)) {
                Ok(()) => {
                    slot.assignments.insert(assignment);
                    self.assignment_to_worker.insert(assignment, worker_id);
                }
                Err(TrySendError::Full(_)) => {
                    debug!(
                        worker_id = %worker_id,
                        tenant_id = %assignment.tenant_id,
                        destination_id = %assignment.destination_id,
                        "Retry worker channel full; assignment deferred to next pass"
                    );
                }
                Err(TrySendError::Closed(_)) => {
                    warn!(
                        worker_id = %worker_id,
                        tenant_id = %assignment.tenant_id,
                        destination_id = %assignment.destination_id,
                        "Failed to send retry assignment; worker channel closed"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(load: usize) -> (Uuid, usize) {
        (Uuid::new_v4(), load)
    }

    #[test]
    fn spawns_when_no_workers() {
        assert_eq!(choose_worker(&[]), None);
    }

    #[test]
    fn picks_least_loaded_under_cap() {
        let a = w(3);
        let b = w(1);
        assert_eq!(choose_worker(&[a, b]), Some(b.0));
    }

    #[test]
    fn spawns_when_all_full_and_below_max() {
        let live = vec![w(DESTS_PER_WORKER_SOFT_CAP); 3];
        assert_eq!(choose_worker(&live), None);
    }

    #[test]
    fn reuses_when_all_full_at_max() {
        let live = vec![w(DESTS_PER_WORKER_SOFT_CAP + 5); MAX_WORKERS];
        assert!(choose_worker(&live).is_some());
    }
}
