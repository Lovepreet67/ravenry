use std::collections::HashSet;
use std::time::Duration;

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Installs the global recorder. Call once, before any metric is emitted.
pub fn install() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("install metrics recorder")
}

/// Exports inbox row counts per (source, status) as a gauge. Labels are
/// only pipeline names and statuses, so cardinality stays tiny.
pub async fn run_inbox_gauges(pool: PgPool, interval: Duration, shutdown: CancellationToken) {
    let mut seen: HashSet<(String, String)> = HashSet::new();

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(interval) => {}
        }

        let rows: Result<Vec<(String, String, i64)>, _> = sqlx::query_as(
            "SELECT source, status, COUNT(*)
               FROM inbox_messages
              WHERE status IN ('pending', 'processing', 'failed', 'quarantined')
              GROUP BY source, status",
        )
        .fetch_all(&pool)
        .await;

        let rows = match rows {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "inbox gauge query failed");
                continue;
            }
        };

        // Zero out combinations that disappeared since the last tick.
        let current: HashSet<(String, String)> = rows
            .iter()
            .map(|(s, st, _)| (s.clone(), st.clone()))
            .collect();
        for (source, status) in seen.difference(&current) {
            ::metrics::gauge!("muninn_inbox_rows", "source" => source.clone(), "status" => status.clone())
                .set(0.0);
        }
        for (source, status, n) in rows {
            ::metrics::gauge!("muninn_inbox_rows", "source" => source, "status" => status)
                .set(n as f64);
        }
        seen = current;
    }
}
