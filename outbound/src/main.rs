pub mod application;
pub mod config;
pub mod model;
pub mod platform;
use std::{env, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tracing::info;

use crate::{
    application::{
        delivery::{http_publisher::HttpPublisher, service::DeliveryService},
        worker::{OutboundWorker, WorkerConfig},
    },
    config::Config,
    platform::{
        projection_store::ProjectionStore,
        topic_registry::{RedisTopicRegistry, TopicRegistry},
    },
};
use streamforge::factory::build_queue;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_owned());

    let config = Config::load(&config_path)
        .with_context(|| format!("failed to load config from {config_path}"))?;

    info!(config_path = %config_path, "configuration loaded");

    // Initialize the queue backend.
    let queue = build_queue(&config.queue).await?;

    // Connect to Redis-backed projections and topic registry.
    let projections = ProjectionStore::connect(&config.projection.redis_url)
        .await
        .context("failed to connect to projection store")?;

    let registry: Arc<dyn TopicRegistry> = Arc::new(
        RedisTopicRegistry::connect(&config.projection.redis_url)
            .await
            .context("failed to connect to topic registry")?,
    );

    // Reuse one HTTP client for all destination deliveries.
    let publisher = HttpPublisher::new().context("failed to initialize HTTP publisher")?;

    let delivery = DeliveryService::new(projections, publisher);

    let worker_config = WorkerConfig {
        consumer_group: config.worker.consumer_group,
        consumer_name: config.worker.consumer_name,
        batch_size: config.worker.batch_size,
        registry_refresh_interval: Duration::from_secs(
            config.worker.registry_refresh_interval_secs,
        ),
        idle_interval: Duration::from_millis(config.worker.idle_interval_ms),
    };

    let worker = OutboundWorker::new(queue, registry, delivery, worker_config);

    info!("starting outbound worker");

    worker
        .run()
        .await
        .context("outbound worker stopped with an error")?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt().init();
}
