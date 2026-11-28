use std::sync::Arc;
use std::time::{Duration, Instant};

use metrics::{counter, histogram};
use mimir::{lables, names};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn, Instrument};

use super::inbox::{mark_processed_tx, InboxMessage, InboxStore};
use super::pipeline::{ProjectError, Projector};
use super::retry::{RetryDecision, RetryPolicy};
use crate::core::inbox::postgres::mark_processed_bulk_tx;
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

#[derive(Debug)]
enum Failure {
    Project(ProjectError),
    LeaseLost,
}
enum BulkOutcome {
    Processed,
    LeaseLost,
    Failed(ProjectError),
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

    pub async fn run(self, shutdown: CancellationToken) {
        info!(source = %self.source, worker = %self.worker_id, "processor started");

        let bulk = self.projector.bulk_project_supported().await;
        info!(source = %self.source, bulk, "projection mode");

        while !shutdown.is_cancelled() {
            let batch = match self
                .store
                .claim_batch(
                    &self.source,
                    &self.worker_id,
                    self.cfg.batch_size,
                    self.cfg.lease,
                )
                .instrument(tracing::info_span!(
                    "inbox.claim_batch",
                    source = %self.source,
                    worker = %self.worker_id,
                    batch_size = self.cfg.batch_size,
                ))
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    counter!(
                        names::MUNINN_DATABASE_ERRORS,
                        lables::PIPELINE => self.source.clone()
                    )
                    .increment(1);
                    error!(source = %self.source, error = %e, "claim failed");
                    sleep_or_cancel(&shutdown, Duration::from_secs(2)).await;
                    continue;
                }
            };

            if batch.is_empty() {
                sleep_or_cancel(&shutdown, self.cfg.poll_interval).await;
                continue;
            }
            let batch_span = tracing::info_span!(
                "inbox.process_batch",
                source = %self.source,
                worker = %self.worker_id,
                claimed = batch.len(),
                bulk,
            );
            async {
                if bulk {
                    self.process_bulk(&batch).await;
                } else {
                    for msg in &batch {
                        self.process_one(msg).await;
                    }
                }
            }
            .instrument(batch_span)
            .await;
        }

        info!(source = %self.source, worker = %self.worker_id, "processor stopped");
    }

    /// Fast path: one tx for the whole batch. On any infrastructure error
    /// the tx is rolled back and we fall back to per-message processing.
    async fn process_bulk(&self, batch: &[InboxMessage]) {
        let started = Instant::now();

        let outcomes = match self.run_bulk_tx(batch).await {
            Ok(o) => o,
            Err(e) => {
                warn!(error = %e, n = batch.len(), "bulk path failed, falling back to per-message");
                counter!(
                    names::MUNINN_MESSAGES_OUTCOME,
                    lables::PIPELINE => self.source.clone(),
                    lables::OUTCOME => "bulk_fallback"
                )
                .increment(1);
                for msg in batch {
                    self.process_one(msg).await;
                }
                return;
            }
        };

        // Tx is committed; record per-message outcomes.
        let per_msg = started.elapsed().as_secs_f64() / batch.len() as f64;
        for (msg, outcome) in batch.iter().zip(outcomes) {
            match outcome {
                BulkOutcome::Processed => {
                    counter!(
                        names::MUNINN_MESSAGES_PROCESSED,
                        lables::PIPELINE => self.source.clone()
                    )
                    .increment(1);
                    counter!(
                        names::MUNINN_MESSAGES_OUTCOME,
                        lables::PIPELINE => self.source.clone(),
                        lables::OUTCOME => "processed"
                    )
                    .increment(1);
                }
                BulkOutcome::LeaseLost => {
                    counter!(
                        names::MUNINN_MESSAGES_OUTCOME,
                        lables::PIPELINE => self.source.clone(),
                        lables::OUTCOME => "lease_lost"
                    )
                    .increment(1);
                    warn!(
                        inbox_id = msg.id,
                        "lease lost, another worker owns this row"
                    );
                }
                BulkOutcome::Failed(e) => {
                    let span = tracing::info_span!(
                        "inbox.bulk_failure",
                        source = %self.source,
                        inbox_id = msg.id,
                        attempts = msg.attempts,
                        deferrals = msg.deferrals,
                        outcome = tracing::field::Empty,
                    );
                    self.handle_failure(msg, e, &span)
                        .instrument(span.clone())
                        .await;
                }
            }
            histogram!(
                names::MUNINN_MESSAGES_PROCESSING_DURATION,
                lables::PIPELINE => self.source.clone()
            )
            .record(per_msg);
        }
    }

    async fn run_bulk_tx(&self, batch: &[InboxMessage]) -> anyhow::Result<Vec<BulkOutcome>> {
        let mut tx = self.pool.begin().await?;

        let results = self.projector.project_bulk(&mut tx, batch).await?;
        if results.len() != batch.len() {
            anyhow::bail!(
                "project_bulk returned {} results for {} messages",
                results.len(),
                batch.len()
            );
        }

        // Only rows whose projection succeeded get marked processed.
        let ok_ids: Vec<i64> = batch
            .iter()
            .zip(&results)
            .filter(|(_, r)| r.is_ok())
            .map(|(m, _)| m.id)
            .collect();

        let marked = mark_processed_bulk_tx(&mut tx, &ok_ids).await?;

        let outcomes = batch
            .iter()
            .zip(results)
            .map(|(msg, res)| match res {
                Ok(()) if marked.contains(&msg.id) => BulkOutcome::Processed,
                Ok(()) => BulkOutcome::LeaseLost,
                Err(e) => BulkOutcome::Failed(e),
            })
            .collect();

        tx.commit().await?;
        Ok(outcomes)
    }

    #[instrument(
        skip(self, msg),
        fields(
            source = %self.source,
            inbox_id = msg.id,
            attempts = msg.attempts,
            deferrals = msg.deferrals,
            outcome = tracing::field::Empty,
        )
    )]
    async fn process_one(&self, msg: &InboxMessage) {
        let span = tracing::Span::current();
        let started = Instant::now();
        match self.run_tx(msg).await {
            Ok(()) => {
                span.record("outcome", "processed");
                counter!(
                    names::MUNINN_MESSAGES_PROCESSED,
                    lables::PIPELINE => self.source.clone()
                )
                .increment(1);
                counter!(
                    names::MUNINN_MESSAGES_OUTCOME,
                    lables::PIPELINE => self.source.clone(),
                    lables::OUTCOME => "processed"
                )
                .increment(1);
                histogram!(
                    names::MUNINN_MESSAGES_PROCESSING_DURATION,
                    lables::PIPELINE => self.source.clone()
                )
                .record(started.elapsed().as_secs_f64());
            }
            Err(Failure::LeaseLost) => {
                span.record("outcome", "lease_lost");
                counter!(
                    names::MUNINN_MESSAGES_OUTCOME,
                    lables::PIPELINE => self.source.clone(),
                    lables::OUTCOME => "lease_lost"
                )
                .increment(1);
                histogram!(
                    names::MUNINN_MESSAGES_PROCESSING_DURATION,
                    lables::PIPELINE => self.source.clone()
                )
                .record(started.elapsed().as_secs_f64());
                warn!("lease lost, another worker owns this row");
            }
            Err(Failure::Project(e)) => {
                self.handle_failure(msg, e, &span).await;
                histogram!(
                    names::MUNINN_MESSAGES_PROCESSING_DURATION,
                    lables::PIPELINE => self.source.clone()
                )
                .record(started.elapsed().as_secs_f64());
            }
        }
    }

    /// BEGIN -> project -> mark processed -> COMMIT.
    /// Any early return drops `tx`, which rolls it back.
    #[instrument(skip(self, msg), level = "info", err(Debug))]
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
    async fn handle_failure(
        &self,
        msg: &InboxMessage,
        err: ProjectError,
        parent_span: &tracing::Span,
    ) {
        let id = msg.id;
        let (kind, result) = match err {
            ProjectError::Transient(e) => match self.retry.on_failure(msg.attempts) {
                RetryDecision::RetryAt(at) => {
                    warn!(error = %e, retry_at = %at, "retrying");
                    ("retry", self.store.schedule_retry(id, &e, at).await)
                }
                RetryDecision::GiveUp => {
                    error!(error = %e, "retries exhausted");
                    ("failed", self.store.mark_failed(id, &e).await)
                }
            },
            ProjectError::Defer(e) => match self.retry.on_defer(msg.deferrals) {
                RetryDecision::RetryAt(at) => {
                    debug!(reason = %e, defer_until = %at, "deferred");
                    ("deferred", self.store.defer(id, &e, at).await)
                }
                RetryDecision::GiveUp => {
                    error!(reason = %e, "dependency never arrived");
                    ("failed", self.store.mark_failed(id, &e).await)
                }
            },
            ProjectError::Invalid(e) => {
                error!(error = %e, "invalid message, quarantined");
                ("quarantined", self.store.mark_quarantined(id, &e).await)
            }
        };

        parent_span.record("outcome", kind);

        counter!(
            names::MUNINN_MESSAGES_OUTCOME,
            lables::PIPELINE => self.source.clone(),
            lables::OUTCOME => kind
        )
        .increment(1);

        // If this write fails (DB down) the row stays `processing` and the
        // reaper returns it to `pending` once the lease expires.
        if let Err(e) = result {
            counter!(
                names::MUNINN_DATABASE_ERRORS,
                lables::PIPELINE => self.source.clone()
            )
            .increment(1);
            error!(error = %e, outcome = kind, "could not record failure outcome");
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
