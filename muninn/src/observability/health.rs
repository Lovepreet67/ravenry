use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use metrics_exporter_prometheus::PrometheusHandle;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

/// Readiness answers "can this instance do its job right now?":
/// - PostgreSQL reachable
/// - every pipeline completed a stream reconciliation recently
///   (which proves Redis and the topic registry are working)
pub struct Health {
    pool: PgPool,
    prom: PrometheusHandle,
    pipelines: Vec<&'static str>,
    stale_after: Duration,
    last_reconcile: Mutex<HashMap<&'static str, Instant>>,
}

impl Health {
    pub fn new(
        pool: PgPool,
        prom: PrometheusHandle,
        pipelines: Vec<&'static str>,
        stale_after: Duration,
    ) -> Arc<Self> {
        Arc::new(Self {
            pool,
            prom,
            pipelines,
            stale_after,
            last_reconcile: Mutex::new(HashMap::new()),
        })
    }

    pub fn mark_reconciled(&self, pipeline: &'static str) {
        self.last_reconcile
            .lock()
            .unwrap()
            .insert(pipeline, Instant::now());
    }
}

pub async fn serve(
    addr: SocketAddr,
    health: Arc<Health>,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    let app = Router::new()
        .route("/healthz", get(|| async { "ok" })) // liveness: process is up
        .route("/readyz", get(ready)) // readiness: can do work
        .route("/metrics", get(metrics))
        .with_state(health);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move { shutdown.cancelled().await })
        .await
}

async fn metrics(State(h): State<Arc<Health>>) -> String {
    h.prom.render()
}

async fn ready(State(h): State<Arc<Health>>) -> (StatusCode, Json<Value>) {
    let mut problems: Vec<String> = Vec::new();

    if sqlx::query("SELECT 1").execute(&h.pool).await.is_err() {
        problems.push("postgres unreachable".into());
    }

    {
        let map = h.last_reconcile.lock().unwrap();
        for p in &h.pipelines {
            match map.get(p) {
                Some(t) if t.elapsed() <= h.stale_after => {}
                Some(_) => problems.push(format!("pipeline {p}: reconciliation stale")),
                None => problems.push(format!("pipeline {p}: not started")),
            }
        }
    }

    if problems.is_empty() {
        (StatusCode::OK, Json(json!({ "status": "ready" })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "problems": problems })),
        )
    }
}
