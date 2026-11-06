mod config;
mod core;
mod error;
mod observability;
mod pipelines;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::core::inbox::PgInboxStore;
use crate::core::{
    run_reaper, Exit, Ingester, IngesterConfig, Processor, ProcessorConfig, RetryPolicy, Supervisor,
};
use crate::observability::{health, metrics, Health};
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Config::load()?;

    let prom = metrics::install();

    // Fail fast: if a dependency is down at startup, exit and let the
    // orchestrator restart us.
    let pool = PgPoolOptions::new()
        .max_connections(cfg.database.max_connections)
        .min_connections(cfg.database.min_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database.url)
        .await?;

    let queue: Arc<dyn streamforge::traits::Queue> =
        streamforge::factory::build_queue(&cfg.queue).await?;
    let registry = Arc::new(
        iam_contracts::topic_registory::RedisTopicRegistry::connect(&cfg.redis.url).await?,
    );

    let state = AppState {
        pool: pool.clone(),
        queue,
        inbox: Arc::new(PgInboxStore::new(pool.clone())),
    };

    let consumer_name = consumer_name("munnin"); //&cfg.queue.consumer_prefix);
    info!(%consumer_name, "muninn starting");

    let pipelines = pipelines::all(registry);
    let names: Vec<&'static str> = pipelines.iter().map(|p| p.name()).collect();

    let reconcile = Duration::from_secs(10); //cfg.queue.reconcile_interval_secs);
    let health = Health::new(
        pool.clone(),
        prom,
        names,
        reconcile * 3 + Duration::from_secs(10),
    );

    let ing_cfg = IngesterConfig {
        consumer_name: consumer_name.clone(),
        batch_size: 100, //cfg.queue.batch_size,
        block_ms: 10,    //cfg.queue.block_ms,
        reconcile_interval: reconcile,
    };
    let proc_cfg = ProcessorConfig {
        batch_size: cfg.inbox.batch_size,
        lease: Duration::from_secs(cfg.inbox.lease_secs),
        poll_interval: Duration::from_millis(cfg.inbox.poll_interval_ms),
    };
    let retry = RetryPolicy {
        max_attempts: cfg.inbox.max_attempts,
        initial_backoff: Duration::from_secs(cfg.inbox.initial_backoff_secs),
        max_backoff: Duration::from_secs(cfg.inbox.max_backoff_secs),
        ..Default::default()
    };

    let mut sup = Supervisor::new();
    let token = sup.token();

    for p in pipelines {
        let name = p.name();

        let ingester = Ingester::new(
            state.queue.clone(),
            state.inbox.clone(),
            p.clone(),
            health.clone(),
            ing_cfg.clone(),
        );
        sup.spawn(format!("ingester:{name}"), ingester.run(token.clone()));

        for i in 0..cfg.workers.processors {
            let processor = Processor::new(
                state.pool.clone(),
                state.inbox.clone(),
                p.projector(),
                retry.clone(),
                name,
                format!("{consumer_name}/{name}/{i}"),
                proc_cfg.clone(),
            );
            sup.spawn(
                format!("processor:{name}:{i}"),
                processor.run(token.clone()),
            );
        }
    }

    // One per service, not per pipeline.
    sup.spawn(
        "reaper",
        run_reaper(
            state.inbox.clone(),
            Duration::from_secs(cfg.inbox.reaper_interval_secs),
            cfg.inbox.max_attempts,
            token.clone(),
        ),
    );
    sup.spawn(
        "inbox-gauges",
        metrics::run_inbox_gauges(
            pool.clone(),
            Duration::from_secs(cfg.observability.gauge_interval_secs),
            token.clone(),
        ),
    );

    let addr: SocketAddr = cfg.observability.http_addr.parse()?;
    let http_token = token.clone();
    sup.spawn("http", async move {
        if let Err(e) = health::serve(addr, health, http_token).await {
            error!(error = %e, "http server failed");
        }
    });

    let exit = sup
        .run(Duration::from_secs(cfg.workers.shutdown_grace_secs))
        .await;
    pool.close().await;

    if exit == Exit::TaskFailed {
        std::process::exit(1);
    }
    info!("muninn stopped cleanly");
    Ok(())
}

/// "<prefix>-<hostname>-<6 random chars>": unique per process.
fn consumer_name(prefix: &str) -> String {
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "local".into());
    let rand = uuid::Uuid::new_v4().simple().to_string();
    return format!("{prefix}-{host}-{}", &rand[..6]);
}
