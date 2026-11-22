use std::{fmt::Display, future::Future, time::Duration};

use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, error, info, warn};

use super::{event::OutboxEvent, store};

const DEFAULT_BATCH_SIZE: i64 = 10;
const MAX_BACKOFF_SECONDS: u64 = 300;

pub async fn run<F, Fut, E>(
    pool: PgPool,
    batch_size: i64,
    poll_interval: Duration,
    shutdown_token: CancellationToken,
    handler: F,
) where
    F: Fn(OutboxEvent) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), E>> + Send,
    E: Display,
{
    let batch_size = if batch_size > 0 {
        batch_size
    } else {
        DEFAULT_BATCH_SIZE
    };

    while !shutdown_token.is_cancelled() {
        let poll_span = tracing::info_span!("iam_outbox_poll", batch_size);
        let events = match store::claim_batch(&pool, batch_size).await {
            Ok(events) => events,
            Err(error) => {
                poll_span.in_scope(|| {
                    error!(%error, "Failed to claim outbox events");
                });
                tokio::time::sleep(poll_interval).await;
                continue;
            }
        };

        if events.is_empty() {
            sleep_or_cancel(poll_interval, &shutdown_token).await;
            continue;
        }
        poll_span.in_scope(|| {
            info!(claimed_count = events.len(), "Claimed outbox events");
        });

        for event in events {
            let event_id = event.id;
            let attempt = event.attempts.max(1) as u32;

            let event_span = tracing::info_span!(
                "outbox_event",
                event_id = %event_id,
                attempt,
                outcome = tracing::field::Empty,
            );

            match handler(event).await {
                Ok(()) => {
                    if let Err(error) = store::mark_processed(&pool, event_id)
                        .instrument(event_span.clone())
                        .await
                    {
                        event_span.record("outcome", "mark_processed_failed");
                        error!(
                            %error,
                            "Failed to mark outbox event as processed"
                        );
                    }
                }
                Err(error) => {
                    let delay = retry_delay_seconds(attempt);
                    event_span.record("outcome", "retrying");
                    event_span.in_scope(|| {
                        warn!(
                            retry_after_seconds = delay,
                            %error,
                            "Outbox event processing failed"
                        );
                    });

                    if let Err(db_error) =
                        store::schedule_retry(&pool, event_id, delay as i64, &error.to_string())
                            .await
                    {
                        event_span.record("outcome", "schedule_retry_failed");
                        error!(
                            %db_error,
                            "Failed to schedule outbox retry"
                        );
                    }
                }
            }
        }
    }
}

fn retry_delay_seconds(attempt: u32) -> u64 {
    let exponent = attempt.saturating_sub(1).min(31);
    2_u64.saturating_pow(exponent).min(MAX_BACKOFF_SECONDS)
}

async fn sleep_or_cancel(duration: Duration, shutdown_token: &CancellationToken) {
    tokio::select! {
        _ = tokio::time::sleep(duration) => {}
        _ = shutdown_token.cancelled() => {}
    }
}
