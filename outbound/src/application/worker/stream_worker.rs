use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use super::runner::{WorkerChannels, WorkerCommand, WorkerRunner, WorkerTask};
use crate::{
    application::delivery::{model::DeliveryRequest, service::DeliveryService},
    config::worker::StreamWorkerConfig,
    platform::{projection_store::ProjectionStore, redis::DeliveryStore},
};
use async_trait::async_trait;
use iam_contracts::{delivery_outcome::EventDestinationError, events::PublishedEventDlq, streams};
use metrics::gauge;
use mimir::names::METRIC_ACTIVE_WORKERS;
use streamforge::{traits::Queue, Message};
use tokio::time::Instant;
use tokio::{
    sync::mpsc::{self, error::TrySendError},
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, field, info, warn, Instrument, Span};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamAssignment {
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
    pub stream: String,
}

#[derive(Debug, Clone)]
pub enum StreamWorkerReport {
    Idle { worker_id: Uuid, idle_passes: u32 },
}

/// Everything a stream worker needs, bundled so the master can spawn workers
/// without an 8-argument constructor.
#[derive(Clone)]
pub struct StreamWorkerDeps {
    pub queue: Arc<dyn Queue>,
    pub projections: Arc<ProjectionStore>,
    pub delivery_store: Arc<DeliveryStore>,
    pub config: Arc<StreamWorkerConfig>,
}

pub struct StreamWorker {
    id: Uuid,
    consumer_name: String,
    queue: Arc<dyn Queue>,
    projections: Arc<ProjectionStore>,
    delivery_store: Arc<DeliveryStore>,
    config: Arc<StreamWorkerConfig>,
}

pub struct StreamWorkerHandle {
    pub id: Uuid,
    cmd_tx: mpsc::Sender<WorkerCommand<StreamAssignment>>,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl StreamWorkerHandle {
    pub fn spawn(
        id: Uuid,
        deps: &StreamWorkerDeps,
        report_tx: mpsc::Sender<StreamWorkerReport>,
        channel_cap: usize,
        token: CancellationToken,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(channel_cap);
        let idle_sleep = Duration::from_secs(deps.config.idle_interval_secs);
        let consumer_name = format!("{}_{}", deps.config.consumer_name_prefix, id);
        let group = deps.config.consumer_group.clone();
        let worker = StreamWorker {
            id,
            consumer_name: consumer_name.clone(),
            queue: deps.queue.clone(),
            projections: deps.projections.clone(),
            delivery_store: deps.delivery_store.clone(),
            config: deps.config.clone(),
        };
        let channels = WorkerChannels { cmd_rx, report_tx };
        let task_token = token.clone();
        let handle = tokio::spawn(async move {
            info!(
                worker_id = %id,
                group = %group,
                consumer = %consumer_name,
                "Stream worker started"
            );

            WorkerRunner::new(worker, channels, idle_sleep, task_token)
                .run()
                .await;
            info!(
                worker_id = %id,
                group = %group,
                consumer = %consumer_name,
                "Stream worker stopped"
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
                "Stream worker join failed"
            );
        }
    }

    /// Gracefully stops this worker before a shared absolute deadline.
    /// The absolute deadline lets the master apply one shutdown budget to all
    /// workers instead of spending `timeout` once per worker.
    pub async fn shutdown_until(self, deadline: Instant) {
        self.token.cancel();
        let id = self.id;
        let mut handle = self.handle;
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, &mut handle).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                warn!(worker_id = %id, error = %err, "Stream worker join failed");
            }
            Err(_) => {
                warn!(worker_id = %id, "Stream worker exceeded shutdown deadline; aborting");
                handle.abort();
                if let Err(err) = handle.await {
                    if !err.is_cancelled() {
                        warn!(worker_id = %id, error = %err, "Stream worker abort join failed");
                    }
                }
            }
        }
    }

    /// Convenience wrapper for callers that have a relative timeout.
    /// Prefer `shutdown_until` when coordinating multiple workers.
    pub async fn shutdown(self, timeout: Duration) {
        self.shutdown_until(Instant::now() + timeout).await;
    }
}

#[async_trait]
impl WorkerTask for StreamWorker {
    type Assignment = StreamAssignment;
    type Report = StreamWorkerReport;

    async fn step(&mut self, assignment: &Self::Assignment) -> Result<bool, anyhow::Error> {
        let batch_span = tracing::info_span!(
            "stream.process_batch",
            worker_id = %self.id,
            tenant_id = %assignment.tenant_id,
            topic_id = %assignment.topic_id,
            stream = %assignment.stream,
            consumer = %self.consumer_name,
            acked = tracing::field::Empty,
            batch_size = tracing::field::Empty,
            destinations_resolved = field::Empty,
        );

        let queue = self.queue.clone();
        let projections = self.projections.clone();
        let delivery_store = self.delivery_store.clone();
        let config = self.config.clone();
        let consumer_name = self.consumer_name.clone();
        async move {
            // ensure that group exist
            if let Err(err) = queue
                .ensure_group(&assignment.stream, &config.consumer_group)
                .await
            {
                error!(err=%err, stream=%&assignment.stream,"Error while creating group");
                return Ok(false);
            }
            let deliveries = queue
                .consume(
                    &assignment.stream,
                    &config.consumer_group,
                    consumer_name.as_str(),
                    config.batch_size,
                    20,
                )
                .await
                .map_err(|err| anyhow::anyhow!("consume failed: {err}"))?;
            Span::current().record("batch_size", deliveries.len());
            if deliveries.is_empty() {
                return Ok(false);
            }
            let destinations: Arc<Vec<_>> = match projections
                .resolve_topic_destinations(assignment.tenant_id, assignment.topic_id)
                .await
            {
                Ok(d) => Arc::new(d.into_iter().map(Arc::new).collect()),
                Err(e) => {
                    error!(err = %e, "Error while fetching destinations for topic");
                    return Err(anyhow::anyhow!("resolve_topic_destinations failed: {e}"));
                }
            };

            Span::current().record("destinations_resolved", destinations.len());
            let destination_id_to_destinations: HashMap<_, _> =
                destinations.iter().cloned().map(|d| (d.id, d)).collect();
            let mut destination_id_to_deliveries: HashMap<Uuid, Vec<Message>> = HashMap::default();
            let mut invalid_events = Vec::new();
            let mut destination_id_to_invalid_deliveries: HashMap<Uuid, Vec<Message>> =
                HashMap::default();

            // NOTE: no cancellation check in this loop on purpose. It is pure CPU
            // (no awaits), and breaking out early would leave the tail of the batch
            // unprocessed while `ack_batch` below acks every delivery, which
            // silently drops those events on shutdown.
            for delivery in &deliveries {
                let event: iam_contracts::events::PublishedEvent =
                    match serde_json::from_slice(&delivery.message.payload) {
                        Ok(event) => event,
                        Err(err) => {
                            warn!(
                                queue_message_id = %delivery.delivery_id,
                                error = %err,
                                "Invalid published event"
                            );
                            let raw_payload =
                                match String::from_utf8(delivery.message.payload.to_vec()) {
                                    Ok(valid_str) => valid_str,
                                    Err(err) => {
                                        // Preserves readable text and replaces invalid UTF-8 sequences with
                                        String::from_utf8_lossy(err.as_bytes()).into_owned()
                                    }
                                };
                            let dlq_bytes = match serde_json::to_vec(&PublishedEventDlq::new(
                                // SAFETY: ID will always be there as inbound is following this
                                Uuid::from_str(delivery.message.id.as_ref().expect("Id must be there").as_str()).expect("Never fails"),
                                Some(raw_payload.clone()),
                                assignment.tenant_id,
                                assignment.topic_id,
                                "Error while deserializing the event".into(),
                            )) {
                                Ok(bytes) => bytes,
                                Err(ser_err) => {
                                    error!(err=%ser_err,"Error while serelizing the dlq message for event");
                                    // Fallback: log or send a minimal plain payload without panicking
                                    continue;
                                }
                            };
                            let msg = Message::new(dlq_bytes);
                            invalid_events.push(msg);
                            continue;
                        }
                    };

                let event_destinations = match &event.destinations {
                    None => destinations.clone(),
                    Some(target_ids) => Arc::new(
                        target_ids
                            .iter()
                            .filter_map(|destination_id| {
                                destination_id_to_destinations.get(destination_id).cloned()
                            })
                            .collect(),
                    ),
                };

                for ed in event_destinations.as_ref() {
                    let request = DeliveryRequest {
                        delivery_id: DeliveryService::delivery_id(event.event_id, ed.id),
                        event_id: event.event_id,
                        tenant_id: event.tenant_id,
                        topic_id: event.topic_id,
                        destination_id: ed.id,
                        payload: event.payload.clone(),
                        destination_config: ed.clone(),
                        curr_attempt: None,
                    };
                    let delivery_req_serialized = match serde_json::to_vec(&request) {
                        Ok(v) => v,
                        Err(e) => {
                            let event_destination_error = EventDestinationError {
                                event_id: event.event_id,
                                tenant_id: event.tenant_id,
                                topic_id: event.topic_id,
                                destination_id: ed.id,
                                error: e.to_string(),
                            };
                            let message = Message::new(
                                serde_json::to_vec(&event_destination_error).expect("Can't fail"),
                            );
                            destination_id_to_invalid_deliveries
                                .entry(ed.id)
                                .or_default()
                                .push(message);
                            continue;
                        }
                    };
                    let delivery_message = Message::new(delivery_req_serialized)
                        .with_id(DeliveryService::delivery_id(event.event_id, ed.id))
                        .with_header("message_type", "delivery");
                    destination_id_to_deliveries
                        .entry(ed.id)
                        .or_default()
                        .push(delivery_message);
                }
            }
            if !destination_id_to_deliveries.is_empty() {
                let active: Vec<(Uuid, Uuid)> = destination_id_to_deliveries
                    .keys()
                    .map(|destination_id| (assignment.tenant_id, *destination_id))
                    .collect();
                delivery_store.mark_destinations_active(&active).await?;
            }

            let mut join_set = JoinSet::new();
            for (destination_id, dest_deliveries) in destination_id_to_deliveries {
                let queue = queue.clone();
                let tenant_id = assignment.tenant_id;
                join_set.spawn(async move {
                    let queue_key = streams::destination_deliveries(tenant_id, destination_id);
                    queue
                        .publish_batch(
                            dest_deliveries
                                .into_iter()
                                .map(|message| (queue_key.as_str(), message))
                                .collect(),
                        )
                        .await
                });
            }

            if !invalid_events.is_empty() {
                let queue_t = queue.clone();
                let tenant_id = assignment.tenant_id;
                let topic_id = assignment.topic_id;
                join_set.spawn(async move {
                    let event_dlq_key = streams::topic_events_dlq(tenant_id, topic_id);
                    queue_t
                        .publish_batch(
                            invalid_events
                                .into_iter()
                                .map(|message| (event_dlq_key.as_str(), message))
                                .collect(),
                        )
                        .await
                });
            }

            for (destination_id, dest_deliveries) in destination_id_to_invalid_deliveries {
                let queue = queue.clone();
                let tenant_id = assignment.tenant_id;
                join_set.spawn(async move {
                    let queue_key = streams::destination_dlq(tenant_id, destination_id);
                    queue
                        .publish_batch(
                            dest_deliveries
                                .into_iter()
                                .map(|message| (queue_key.as_str(), message))
                                .collect(),
                        )
                        .await
                });
            }

            let mut tasks_failed = 0usize;
            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(Ok(_)) => {}
                    Ok(Err(err)) => {
                        tasks_failed += 1;
                        warn!(error = %err, "Publish batch failed");
                    }
                    Err(join_err) => {
                        tasks_failed += 1;
                        warn!(error = %join_err, "Stream publish task panicked or failed");
                    }
                }
            }

            if tasks_failed > 0 {
                Span::current().record("acked", false);
                // Return Err (not Ok(true)) so the runner treats this as a failed
                // pass and backs off instead of hot-looping on a persistent failure.
                // The batch stays unacked and is redelivered; deterministic delivery
                // ids make republishing idempotent downstream.
                return Err(anyhow::anyhow!(
                    "{tasks_failed} publish task(s) failed; batch left unacked"
                ));
            }

            match queue
                .ack_batch(
                    &assignment.stream,
                    &config.consumer_group,
                    deliveries
                        .iter()
                        .map(|d| d.delivery_id.as_str())
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
                .await
            {
                Ok(()) => {
                    Span::current().record("acked", true);
                    debug!("Source event ACKed");
                }
                Err(err) => {
                    Span::current().record("acked", false);
                    return Err(anyhow::anyhow!("ack failed: {err}"));
                }
            }
            Ok(true)
        }
        .instrument(batch_span)
        .await
    }

    fn on_idle(&mut self, idle_passes: u32) -> Option<Self::Report> {
        Some(StreamWorkerReport::Idle {
            worker_id: self.id,
            idle_passes,
        })
    }
}

const STREAM_WORKER_CHANNEL_CAP: usize = 250;
const STREAM_REPORT_CHANNEL_CAP: usize = 250;
const MAX_WORKERS: usize = 10;
const STREAMS_PER_WORKER_SOFT_CAP: usize = 10;

struct WorkerSlot {
    handle: StreamWorkerHandle,
    assignments: HashSet<StreamAssignment>,
}

pub struct StreamWorkerMaster {
    report_rx: mpsc::Receiver<StreamWorkerReport>,
    report_tx: mpsc::Sender<StreamWorkerReport>,
    workers: HashMap<Uuid, WorkerSlot>,
    assignment_to_worker: HashMap<StreamAssignment, Uuid>,
    deps: StreamWorkerDeps,
}

/// Pure placement decision. `live` is (worker_id, current_load) for live workers
/// only. Returns the worker to use, or `None` if a new one should be spawned.
fn choose_worker(live: &[(Uuid, usize)]) -> Option<Uuid> {
    let &(id, load) = live.iter().min_by_key(|(_, load)| *load)?;
    (load < STREAMS_PER_WORKER_SOFT_CAP || live.len() >= MAX_WORKERS).then_some(id)
}

impl StreamWorkerMaster {
    pub fn new(deps: StreamWorkerDeps) -> Self {
        let (report_tx, report_rx) = mpsc::channel(STREAM_REPORT_CHANNEL_CAP);
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
                StreamWorkerReport::Idle {
                    worker_id,
                    idle_passes,
                } => {
                    debug!(
                        worker_id = %worker_id,
                        idle_passes = idle_passes,
                        "Stream worker idle"
                    );
                }
            }
        }
    }

    fn spawn_worker(&mut self, shutdown_token: &CancellationToken) -> Uuid {
        let id = Uuid::new_v4();
        let handle = StreamWorkerHandle::spawn(
            id,
            &self.deps,
            self.report_tx.clone(),
            STREAM_WORKER_CHANNEL_CAP,
            shutdown_token.child_token(),
        );
        debug!(worker_id = %id, "Spawned stream worker");
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
        active: &HashSet<StreamAssignment>,
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
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "stream").set(live as f64);
    }

    /// Cancels every worker and waits until one shared shutdown deadline.
    /// All workers are signalled first; then each join consumes only the
    /// remaining time before the same deadline. Total shutdown time is thus
    /// bounded by approximately `timeout`, not `worker_count * timeout`.
    pub async fn shutdown(&mut self, timeout: Duration) {
        self.shutdown_until(Instant::now() + timeout).await;
    }

    /// Absolute-deadline variant for services that coordinate shutdown across
    /// several components.
    pub async fn shutdown_until(&mut self, deadline: Instant) {
        let slots: Vec<WorkerSlot> = self.workers.drain().map(|(_, slot)| slot).collect();
        self.assignment_to_worker.clear();
        // Signal all workers before awaiting any one of them.
        for slot in &slots {
            slot.handle.token.cancel();
        }
        for slot in slots {
            slot.handle.shutdown_until(deadline).await;
        }
        gauge!(METRIC_ACTIVE_WORKERS, mimir::lables::WORKER_TYPE => "stream").set(0.0);
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
                debug!(worker_id = %worker_id, "Retiring finished stream worker");
                slot.handle.reap().await;
            }
        }
    }

    /// Sends `Remove` for assignments that are no longer active. Maps are only
    /// updated after a successful send, so a full channel just defers removal.
    fn remove_inactive(&mut self, active: &HashSet<StreamAssignment>) {
        let stale: Vec<StreamAssignment> = self
            .assignment_to_worker
            .keys()
            .filter(|a| !active.contains(*a))
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
                        topic_id = %assignment.topic_id,
                       "Stream worker channel full; deferring removal"
                    );
                }
                Err(TrySendError::Closed(_)) => {
                    // Worker is gone; reap_finished clears its state next pass.
                    debug!(worker_id = %worker_id, "Stream worker channel closed during removal");
                }
            }
        }
    }

    async fn assign_unassigned(
        &mut self,
        active: &HashSet<StreamAssignment>,
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
                        topic_id = %assignment.topic_id,
                        "Stream worker channel full; assignment deferred to next pass"
                    );
                }
                Err(TrySendError::Closed(_)) => {
                    warn!(
                        worker_id = %worker_id,
                        tenant_id = %assignment.tenant_id,
                        topic_id = %assignment.topic_id,
                        "Failed to send stream assignment; worker channel closed"
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
        let live = vec![w(STREAMS_PER_WORKER_SOFT_CAP); 3];
        assert_eq!(choose_worker(&live), None);
    }

    #[test]
    fn reuses_when_all_full_at_max() {
        let live = vec![w(STREAMS_PER_WORKER_SOFT_CAP + 5); MAX_WORKERS];
        assert!(choose_worker(&live).is_some());
    }
}
