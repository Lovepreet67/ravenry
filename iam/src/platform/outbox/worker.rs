use std::{fmt::Display, future::Future, time::Duration};

use sqlx::PgPool;
use tracing::{error, warn};

use super::{event::OutboxEvent, store};

const DEFAULT_BATCH_SIZE: i64 = 10;
const MAX_BACKOFF_SECONDS: u64 = 300;

pub async fn run<F, Fut, E>(pool: PgPool, batch_size: i64, poll_interval: Duration, handler: F)
where
    F: Fn(OutboxEvent) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), E>> + Send,
    E: Display,
{
    let batch_size = if batch_size > 0 {
        batch_size
    } else {
        DEFAULT_BATCH_SIZE
    };

    loop {
        let events = match store::claim_batch(&pool, batch_size).await {
            Ok(events) => events,
            Err(error) => {
                error!(%error, "Failed to claim outbox events");
                tokio::time::sleep(poll_interval).await;
                continue;
            }
        };

        if events.is_empty() {
            tokio::time::sleep(poll_interval).await;
            continue;
        }

        for event in events {
            let event_id = event.id;
            let attempt = event.attempts.max(1) as u32;

            match handler(event).await {
                Ok(()) => {
                    if let Err(error) = store::mark_processed(&pool, event_id).await {
                        // The projection may already have been written.
                        // Reprocessing must therefore remain safe.
                        error!(
                            event_id,
                            %error,
                            "Failed to mark outbox event as processed"
                        );
                    }
                }
                Err(error) => {
                    let delay = retry_delay_seconds(attempt);

                    warn!(
                        event_id,
                        attempt,
                        retry_after_seconds = delay,
                        %error,
                        "Outbox event processing failed"
                    );

                    if let Err(db_error) =
                        store::schedule_retry(&pool, event_id, delay as i64, &error.to_string())
                            .await
                    {
                        error!(
                            event_id,
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
