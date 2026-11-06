pub mod application;
pub mod config;
pub mod model;
pub mod platform;

use std::{env, sync::Arc};

use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use iam_contracts::topic_registory::{RedisTopicRegistry, TopicRegistry};

use streamforge::factory::build_queue;

use crate::{
    application::{
        delivery::{http_publisher::HttpPublisher, retry::RetryPolicy, service::DeliveryService},
        worker::supervisor::WorkerSupervisor,
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

    let projections = Arc::new(
        ProjectionStore::connect(redis_url)
            .await
            .context("failed to connect to projection store")?,
    );

    let delivery_store = Arc::new(
        DeliveryStore::connect(redis_url)
            .await
            .context("failed to connect to delivery store")?,
    );

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

    let publisher = Arc::new(HttpPublisher::new().context("failed to initialize HTTP publisher")?);

    let retry_policy = Arc::new(RetryPolicy::default());

    let delivery_service = Arc::new(DeliveryService::new(
        projections.clone(),
        delivery_store.clone(),
        publisher.clone(),
        retry_policy.clone(),
    ));

    // ---------------------------------------------------------

    let shutdown_token = CancellationToken::new();
    let supervisor = Arc::new(WorkerSupervisor::new(
        Arc::new(config.worker.into()),
        registry.clone(),
        delivery_service.clone(),
        projections.clone(),
        delivery_store.clone(),
        queue.clone(),
        shutdown_token,
    ));

    // ---------------------------------------------------------
    // Run service
    // ---------------------------------------------------------

    info!("Starting outbound service");

    let mut supervisor_task = tokio::spawn(async move { supervisor.run().await });

    tokio::select! {
        result = &mut supervisor_task => {

            result.context("background worker supervisor panicked")?;
            error!("Background worker supervisor unexpectedly stopped");
        }

        signal = tokio::signal::ctrl_c() => {
            signal.context("failed to listen for shutdown signal")?;

            info!("Shutdown signal received");

            supervisor_task.abort();

            let _ = supervisor_task.await;
        }
    }

    info!("Outbound service stopped");

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt().with_target(true).init();
}
