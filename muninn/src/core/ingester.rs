use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use metrics::{counter, gauge};
use mimir::{lables, names};
use streamforge::traits::Queue;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn, Instrument};

use super::inbox::{InboxStatus, InboxStore, NewInboxMessage};
use super::pipeline::Pipeline;
use crate::error::Result;
use crate::observability::Health;

#[derive(Debug, Clone)]
pub struct IngesterConfig {
    /// Unique per process (hostname + suffix). The group is shared.
    pub consumer_name: String,
    pub batch_size: usize,
    /// Bounds shutdown latency, since cancellation is checked between reads.
    pub block_ms: usize,
    pub reconcile_interval: Duration,
}

struct Reader {
    token: CancellationToken,
    handle: JoinHandle<()>,
}

pub struct Ingester {
    queue: Arc<dyn Queue>,
    store: Arc<dyn InboxStore>,
    pipeline: Arc<dyn Pipeline>,
    health: Arc<Health>,
    group: String,
    cfg: IngesterConfig,
}

impl Ingester {
    pub fn new(
        queue: Arc<dyn Queue>,
        store: Arc<dyn InboxStore>,
        pipeline: Arc<dyn Pipeline>,
        health: Arc<Health>,
        cfg: IngesterConfig,
    ) -> Arc<Self> {
        let group = pipeline.consumer_group();
        Arc::new(Self {
            queue,
            store,
            pipeline,
            health,
            group,
            cfg,
        })
    }

    pub async fn run(self: Arc<Self>, shutdown: CancellationToken) {
        let name = self.pipeline.name();
        info!(pipeline = name, "ingester started");
        let mut readers: HashMap<String, Reader> = HashMap::new();

        while !shutdown.is_cancelled() {
            match self.reconcile(&mut readers, &shutdown).await {
                Ok(()) => self.health.mark_reconciled(name),
                Err(e) => warn!(pipeline = name, error = %e, "reconcile failed"),
            }
            gauge!("muninn_active_streams", "pipeline" => name).set(readers.len() as f64);

            sleep_or_cancel(&shutdown, self.cfg.reconcile_interval).await;
        }

        for (_, r) in readers.drain() {
            r.token.cancel();
            let _ = r.handle.await;
        }
        info!(pipeline = name, "ingester stopped");
    }

    #[instrument(
        skip(self, readers, shutdown),
        level = "info",
        fields(pipeline = %self.pipeline.name(), group = %self.group)
    )]
    async fn reconcile(
        self: &Arc<Self>,
        readers: &mut HashMap<String, Reader>,
        shutdown: &CancellationToken,
    ) -> Result<()> {
        let wanted: HashSet<String> = self
            .pipeline
            .source()
            .streams()
            .await?
            .into_iter()
            .collect();

        readers.retain(|name, r| {
            if !wanted.contains(name) {
                info!(stream = %name, "stream no longer active, stopping reader");
                r.token.cancel();
                return false;
            }
            if r.handle.is_finished() {
                warn!(stream = %name, "reader exited unexpectedly, will restart");
                return false;
            }
            true
        });

        for name in wanted {
            if readers.contains_key(&name) {
                continue;
            }
            // Idempotent. Must create the group at the START of the stream
            // (XGROUP CREATE ... 0 MKSTREAM) so history isn't skipped.
            if let Err(e) = self.queue.ensure_group(&name, &self.group).await {
                warn!(stream = %name, error = %e, "ensure_group failed, retrying next tick");
                continue;
            }
            info!(stream = %name, group = %self.group, "starting reader");
            let token = shutdown.child_token();
            let handle = tokio::spawn(self.clone().read_loop(name.clone(), token.clone()));
            readers.insert(name, Reader { token, handle });
        }
        Ok(())
    }

    async fn read_loop(self: Arc<Self>, stream: String, token: CancellationToken) {
        let pipeline = self.pipeline.name();

        while !token.is_cancelled() {
            let deliveries = match self
                .queue
                .consume(
                    &stream,
                    &self.group,
                    &self.cfg.consumer_name,
                    self.cfg.batch_size,
                    self.cfg.block_ms,
                )
                .await
            {
                Ok(d) => d,
                Err(e) => {
                    warn!(pipeline, stream = %stream, error = %e, "consume failed");
                    sleep_or_cancel(&token, Duration::from_secs(1)).await;
                    continue;
                }
            };
            if deliveries.is_empty() {
                continue;
            }
            let batch_span = tracing::info_span!(
                "ingester.batch",
                pipeline,
                stream = %stream,
                group = %self.group,
                consumer = %self.cfg.consumer_name,
                deliveries = deliveries.len(),
                malformed = tracing::field::Empty,
                acked = tracing::field::Empty,
            );
            async {
                counter!(names::QUEUE_DELIVERIES_RECEIVED, lables::PIPELINE => pipeline)
                    .increment(deliveries.len() as u64);

                let rows: Vec<NewInboxMessage> = deliveries
                    .iter()
                    .map(|d| {
                        self.pipeline
                            .prepare(&stream, queue_message_id(d), &raw_bytes(d))
                    })
                    .collect();

                let malformed = rows
                    .iter()
                    .filter(|r| r.status == InboxStatus::Quarantined)
                    .count();
                tracing::Span::current().record("malformed", malformed);
                if malformed > 0 {
                    counter!(names::QUEUE_MALFORMED_DELIVERIES_RECEIVED,lables::PIPELINE  => pipeline)
                    .increment(malformed as u64);
                }

                // Durable first. DB down => retry in place (backpressure), never ACK.
                if !self.insert_with_retry(&rows, &token).await {
                    // cancelled before commit: NOT acked, stays pending in Redis
                    tracing::Span::current().record("acked", false);
                    return;
                }
                counter!(names::MUNINN_INBOX_MESSAGES, lables::PIPELINE => pipeline)
                    .increment(rows.len() as u64);

                // Inbox committed. ACK failures are harmless: redelivery is deduped.
                let delivery_ids: Vec<_> = deliveries.iter().map(|d| queue_message_id(d)).collect();
                match self
                    .queue
                    .ack_batch(&stream, &self.group, &delivery_ids)
                    .await
                {
                    Ok(()) => {
                        tracing::Span::current().record("acked", true);
                    }
                    Err(e) => {
                        tracing::Span::current().record("acked", false);
                        warn!(error = %e, "ack failed, will be redelivered");
                    }
                }
                debug!(n = deliveries.len(), "inboxed and acked");
            }
            .instrument(batch_span)
            .await;
        }
    }

    #[instrument(
        skip(self, rows, token),
        level = "info",
        fields(pipeline = %self.pipeline.name(), batch_len = rows.len())
    )]
    async fn insert_with_retry(&self, rows: &[NewInboxMessage], token: &CancellationToken) -> bool {
        let mut delay = Duration::from_secs(1);
        loop {
            match self.store.insert_batch(rows).await {
                Ok(_) => return true,
                Err(e) => {
                    counter!(names::MUNINN_INBOX_DATABASE_ERRORS,lables::PIPELINE=> self.pipeline.name())
                        .increment(1);
                    error!(error = %e, "inbox insert failed, retrying");
                    if token.is_cancelled() {
                        return false;
                    }
                    sleep_or_cancel(token, delay).await;
                    delay = (delay * 2).min(Duration::from_secs(30));
                }
            }
        }
    }
}

async fn sleep_or_cancel(token: &CancellationToken, d: Duration) {
    tokio::select! {
        _ = token.cancelled() => {}
        _ = tokio::time::sleep(d) => {}
    }
}

// ---------------------------------------------------------------------
// StreamForge adapter: the ONLY code that touches the real
// `Delivery` / `Message` layout. ADJUST both.
// ---------------------------------------------------------------------

/// The queue-level entry ID (e.g. Redis "1700000000000-0").
fn queue_message_id(d: &streamforge::Delivery) -> &str {
    return &d.delivery_id;
}

/// The original message bytes exactly as published.
fn raw_bytes(d: &streamforge::Delivery) -> Vec<u8> {
    d.message.payload.clone() // ASSUMED field
}
