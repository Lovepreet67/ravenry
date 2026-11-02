pub mod application;
pub mod config;
pub mod model;
pub mod platform;

use std::{env, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use iam_contracts::topic_registory::{RedisTopicRegistry, TopicRegistry};

use streamforge::factory::build_queue;

use crate::{
    application::{
        delivery::{http_publisher::HttpPublisher, retry::RetryPolicy, service::DeliveryService},
        worker::{
            dlq_publisher::DlqPublisher,
            outbound_worker::{OutboundWorkerSupervisor, WorkerConfig as OutboundWorkerConfig},
            retry_worker::RetryWorker,
            supervisor::WorkerSupervisor,
        },
    },
    config::Config,
    platform::{projection_store::ProjectionStore, redis::delivery_store::DeliveryStore},
};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_owned());

    let config = Config::load(&config_path)
        .with_context(|| format!("failed to load config from {config_path}"))?;

    info!(config_path = %config_path, "Configuration loaded");

    // ---------------------------------------------------------
    // Infrastructure
    // ---------------------------------------------------------

    let queue = build_queue(&config.queue)
        .await
        .context("failed to initialize queue backend")?;

    let redis_url = &config.projection.redis_url;

    let projections = ProjectionStore::connect(redis_url)
        .await
        .context("failed to connect to projection store")?;

    let delivery_store = DeliveryStore::connect(redis_url)
        .await
        .context("failed to connect to delivery store")?;

    let registry: Arc<dyn TopicRegistry> = Arc::new(
        RedisTopicRegistry::connect(redis_url)
            .await
            .context("failed to connect to topic registry")?,
    );

    projections
        .health_check()
        .await
        .context("projection Redis health check failed")?;

    delivery_store
        .health_check()
        .await
        .context("delivery Redis health check failed")?;

    // ---------------------------------------------------------
    // Delivery application
    // ---------------------------------------------------------

    let publisher = HttpPublisher::new().context("failed to initialize HTTP publisher")?;

    let retry_policy = RetryPolicy::default();

    let delivery_service = DeliveryService::new(
        projections.clone(),
        delivery_store.clone(),
        publisher,
        retry_policy,
    );

    // ---------------------------------------------------------
    // Event consumer
    // ---------------------------------------------------------

    let outbound_worker_config = OutboundWorkerConfig {
        consumer_group: config.worker.consumer_group.clone(),
        consumer_name: config.worker.consumer_name.clone(),
        batch_size: config.worker.batch_size,
        registry_refresh_interval: Duration::from_secs(
            config.worker.registry_refresh_interval_secs,
        ),
        idle_interval: Duration::from_millis(config.worker.idle_interval_ms),
        workers_per_stream: 1,
    };

    let outbound_worker = OutboundWorkerSupervisor::new(
        queue.clone(),
        registry.clone(),
        Arc::new(delivery_service.clone()),
        outbound_worker_config,
    );

    // ---------------------------------------------------------
    // Retry worker
    // ---------------------------------------------------------

    let retry_worker = RetryWorker::new(
        registry.clone(),
        projections.clone(),
        delivery_store.clone(),
        delivery_service,
        config.worker.batch_size,
    );

    // ---------------------------------------------------------
    // DLQ recovery publisher
    // ---------------------------------------------------------

    let dlq_publisher = DlqPublisher::new(delivery_store, queue, config.worker.batch_size);

    // ---------------------------------------------------------
    // Background worker supervisor
    // ---------------------------------------------------------

    let supervisor = WorkerSupervisor::new(
        retry_worker,
        dlq_publisher,
        Duration::from_secs(config.worker.retry_poll_interval_secs),
        Duration::from_secs(config.worker.dlq_poll_interval_secs),
    );

    // ---------------------------------------------------------
    // Run service
    // ---------------------------------------------------------

    info!("Starting outbound service");
    let token = CancellationToken::new();

    let mut outbound_task = tokio::spawn(async move { Arc::new(outbound_worker).run(token).await });

    let mut supervisor_task = tokio::spawn(async move { supervisor.run().await });

    tokio::select! {
        result = &mut outbound_task => {
            supervisor_task.abort();

            result.context("outbound worker task panicked")?

        }

        result = &mut supervisor_task => {
            outbound_task.abort();

            result.context("background worker supervisor panicked")?;
            error!("Background worker supervisor unexpectedly stopped");
        }

        signal = tokio::signal::ctrl_c() => {
            signal.context("failed to listen for shutdown signal")?;

            info!("Shutdown signal received");

            outbound_task.abort();
            supervisor_task.abort();

            let _ = outbound_task.await;
            let _ = supervisor_task.await;
        }
    }

    info!("Outbound service stopped");

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt().with_target(true).init();
}
