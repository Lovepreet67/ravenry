use std::sync::Arc;
use std::time::{Duration, Instant};

use metrics::{counter, histogram};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use super::inbox::{mark_processed_tx, InboxMessage, InboxStore};
use super::pipeline::{ProjectError, Projector};
use super::retry::{RetryDecision, RetryPolicy};
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    pub batch_size: i64,
    /// Must exceed the worst-case time to process one batch.
    pub lease: Duration,
    /// Sleep when the inbox has nothing due.
    pub poll_interval: Duration,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            lease: Duration::from_secs(60),
            poll_interval: Duration::from_millis(500),
        }
    }
}

enum Failure {
    Project(ProjectError),
    LeaseLost,
}

pub struct Processor {
    pool: PgPool,
    store: Arc<dyn InboxStore>,
    projector: Arc<dyn Projector>,
    retry: RetryPolicy,
    source: String,    // pipeline name == inbox_messages.source
    worker_id: String, // unique per task, stored in locked_by
    cfg: ProcessorConfig,
}

impl Processor {
    pub fn new(
        pool: PgPool,
        store: Arc<dyn InboxStore>,
        projector: Arc<dyn Projector>,
        retry: RetryPolicy,
        source: impl Into<String>,
        worker_id: impl Into<String>,
        cfg: ProcessorConfig,
    ) -> Self {
        Self {
            pool,
            store,
            projector,
            retry,
            source: source.into(),
            worker_id: worker_id.into(),
            cfg,
        }
    }

    /// Runs until `shutdown` fires. A claimed batch is always finished
    /// before exiting, so no rows are left leased.
    pub async fn run(self, shutdown: CancellationToken) {
        info!(source = %self.source, worker = %self.worker_id, "processor started");

        while !shutdown.is_cancelled() {
            let batch = match self
                .store
                .claim_batch(
                    &self.source,
                    &self.worker_id,
                    self.cfg.batch_size,
                    self.cfg.lease,
                )
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    error!(source = %self.source, error = %e, "claim failed");
                    sleep_or_cancel(&shutdown, Duration::from_secs(2)).await;
                    continue;
                }
            };

            if batch.is_empty() {
                sleep_or_cancel(&shutdown, self.cfg.poll_interval).await;
                continue;
            }

            debug!(source = %self.source, n = batch.len(), "claimed batch");
            for msg in &batch {
                self.process_one(msg).await;
            }
        }

        info!(source = %self.source, worker = %self.worker_id, "processor stopped");
    }

    async fn process_one(&self, msg: &InboxMessage) {
        let started = Instant::now();
        match self.run_tx(msg).await {
            Ok(()) => {
                counter!("muninn_messages_processed_total", "pipeline" => self.source.clone())
                    .increment(1);
                histogram!("muninn_processing_duration_seconds", "pipeline" => self.source.clone())
                    .record(started.elapsed().as_secs_f64());
            }
            Err(Failure::LeaseLost) => {
                warn!(
                    inbox_id = msg.id,
                    "lease lost, another worker owns this row"
                );
            }
            Err(Failure::Project(e)) => self.handle_failure(msg, e).await,
        }
    }

    /// BEGIN -> project -> mark processed -> COMMIT.
    /// Any early return drops `tx`, which rolls it back.
    async fn run_tx(&self, msg: &InboxMessage) -> Result<(), Failure> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Failure::Project(e.into()))?;

        self.projector
            .project(&mut tx, msg)
            .await
            .map_err(Failure::Project)?;

        match mark_processed_tx(&mut tx, msg.id).await {
            Ok(()) => {}
            Err(Error::LeaseLost(_)) => return Err(Failure::LeaseLost),
            Err(e) => return Err(Failure::Project(e.into())),
        }

        tx.commit().await.map_err(|e| Failure::Project(e.into()))?;
        Ok(())
    }

    /// The transaction is already rolled back here, so these writes use
    /// the pool and are independent of the failed work.
    async fn handle_failure(&self, msg: &InboxMessage, err: ProjectError) {
        let id = msg.id;
        let (kind, result) = match err {
            ProjectError::Transient(e) => match self.retry.on_failure(msg.attempts) {
                RetryDecision::RetryAt(at) => {
                    warn!(inbox_id = id, attempts = msg.attempts, error = %e, "retrying");
                    ("retry", self.store.schedule_retry(id, &e, at).await)
                }
                RetryDecision::GiveUp => {
                    error!(inbox_id = id, error = %e, "retries exhausted");
                    ("failed", self.store.mark_failed(id, &e).await)
                }
            },
            ProjectError::Defer(e) => match self.retry.on_defer(msg.deferrals) {
                RetryDecision::RetryAt(at) => {
                    debug!(inbox_id = id, deferrals = msg.deferrals, reason = %e, "deferred");
                    ("deferred", self.store.defer(id, &e, at).await)
                }
                RetryDecision::GiveUp => {
                    error!(inbox_id = id, reason = %e, "dependency never arrived");
                    ("failed", self.store.mark_failed(id, &e).await)
                }
            },
            ProjectError::Invalid(e) => {
                error!(inbox_id = id, error = %e, "invalid message, quarantined");
                ("quarantined", self.store.mark_quarantined(id, &e).await)
            }
        };

        counter!(
            "muninn_message_outcomes_total",
            "pipeline" => self.source.clone(),
            "outcome" => kind
        )
        .increment(1);

        // If this write fails (DB down) the row stays `processing` and the
        // reaper returns it to `pending` once the lease expires.
        if let Err(e) = result {
            counter!("muninn_database_errors_total", "pipeline" => self.source.clone())
                .increment(1);
            error!(inbox_id = id, error = %e, "could not record failure outcome");
        }
    }
}

/// Returns expired leases to `pending`. Run one per service, not per pipeline.
pub async fn run_reaper(
    store: Arc<dyn InboxStore>,
    interval: Duration,
    max_attempts: i32,
    shutdown: CancellationToken,
) {
    while !shutdown.is_cancelled() {
        sleep_or_cancel(&shutdown, interval).await;
        match store.reap_stale(max_attempts).await {
            Ok(0) => {}
            Ok(n) => warn!(reaped = n, "recovered stale inbox leases"),
            Err(e) => error!(error = %e, "reaper failed"),
        }
    }
}

async fn sleep_or_cancel(shutdown: &CancellationToken, d: Duration) {
    tokio::select! {
        _ = shutdown.cancelled() => {}
        _ = tokio::time::sleep(d) => {}
    }
}
