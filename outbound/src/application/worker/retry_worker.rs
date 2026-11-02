use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::Context;
use iam_contracts::topic_registory::TopicRegistry;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    application::delivery::service::DeliveryService,
    platform::{projection_store::ProjectionStore, redis::delivery_store::DeliveryStore},
};

#[derive(Debug, Default)]
pub struct RetryWorkerStats {
    pub destinations_scanned: usize,
    pub retries_found: usize,
    pub retries_executed: usize,
    pub missing_requests: usize,
    pub failures: usize,
}

pub struct RetryWorker {
    registry: Arc<dyn TopicRegistry>,
    projections: ProjectionStore,
    store: DeliveryStore,
    delivery_service: DeliveryService,
    batch_size: usize,
}

impl RetryWorker {
    pub fn new(
        registry: Arc<dyn TopicRegistry>,
        projections: ProjectionStore,
        store: DeliveryStore,
        delivery_service: DeliveryService,
        batch_size: usize,
    ) -> Self {
        Self {
            registry,
            projections,
            store,
            delivery_service,
            batch_size: batch_size.max(1),
        }
    }

    /// Discovers due retries and executes them.
    ///
    /// Each delivery is retried independently. A failure for one delivery
    /// does not prevent other due deliveries from being processed.
    pub async fn run_once(&self) -> anyhow::Result<RetryWorkerStats> {
        let topics = self
            .registry
            .list_active()
            .await
            .context("failed to list active topics")?;

        let mut destinations = HashSet::<(Uuid, Uuid)>::new();

        for topic in topics {
            let resolved = match self
                .projections
                .resolve_topic_destinations(topic.tenant_id, topic.topic_id)
                .await
            {
                Ok(destinations) => destinations,
                Err(error) => {
                    warn!(
                        tenant_id = %topic.tenant_id,
                        topic_id = %topic.topic_id,
                        error = %error,
                        "Failed to resolve destinations for retry discovery"
                    );
                    continue;
                }
            };

            for destination in resolved {
                if destination.config.status == "active" {
                    destinations.insert((topic.tenant_id, destination.id));
                }
            }
        }

        let mut stats = RetryWorkerStats {
            destinations_scanned: destinations.len(),
            ..Default::default()
        };

        for (tenant_id, destination_id) in destinations {
            let delivery_ids = match self
                .store
                .claim_due_retries(tenant_id, destination_id, self.batch_size)
                .await
            {
                Ok(ids) => ids,
                Err(error) => {
                    stats.failures += 1;
                    error!(
                        tenant_id = %tenant_id,
                        destination_id = %destination_id,
                        error = %error,
                        "Failed to discover due retries"
                    );
                    continue;
                }
            };

            stats.retries_found += delivery_ids.len();

            for delivery_id in delivery_ids {
                let request = match self
                    .store
                    .get_delivery_request(tenant_id, destination_id, delivery_id)
                    .await
                {
                    Ok(Some(request)) => request,

                    Ok(None) => {
                        stats.missing_requests += 1;
                        warn!(
                            tenant_id = %tenant_id,
                            destination_id = %destination_id,
                            delivery_id = %delivery_id,
                            "Retry request is missing from Redis"
                        );
                        continue;
                    }

                    Err(error) => {
                        stats.failures += 1;
                        error!(
                            delivery_id = %delivery_id,
                            error = %error,
                            "Failed to load retry request"
                        );
                        continue;
                    }
                };

                match self.delivery_service.retry_delivery(request).await {
                    Ok(()) => {
                        stats.retries_executed += 1;
                        debug!(
                            delivery_id = %delivery_id,
                            "Retry execution completed"
                        );
                    }
                    Err(error) => {
                        stats.failures += 1;
                        error!(
                            delivery_id = %delivery_id,
                            error = %error,
                            "Retry execution failed"
                        );
                    }
                }
            }
        }

        info!(
            destinations_scanned = stats.destinations_scanned,
            retries_found = stats.retries_found,
            retries_executed = stats.retries_executed,
            missing_requests = stats.missing_requests,
            failures = stats.failures,
            "Retry worker iteration completed"
        );

        Ok(stats)
    }

    /// Runs continuously until the task is cancelled.
    pub async fn run(&self, poll_interval: Duration) {
        let mut interval = tokio::time::interval(poll_interval);

        loop {
            interval.tick().await;

            if let Err(error) = self.run_once().await {
                error!(
                    error = %error,
                    "Retry worker iteration failed"
                );
            }
        }
    }
}
