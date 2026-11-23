use std::{sync::Arc, time::Duration};

use anyhow::{bail, Context};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::{
    application::delivery::{
        http_publisher::HttpPublisher,
        model::{AcquireResult, CompleteAttemptResult, DeliveryRequest},
        retry::{RetryDecision, RetryPolicy},
    },
    platform::projection_store::ProjectionStore,
    platform::redis::delivery_store::DeliveryStore,
};

const DELIVERY_LEASE: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct DeliveryService {
    projections: Arc<ProjectionStore>,
    store: Arc<DeliveryStore>,
    publisher: Arc<HttpPublisher>,
}

impl DeliveryService {
    pub fn new(
        projections: Arc<ProjectionStore>,
        store: Arc<DeliveryStore>,
        publisher: Arc<HttpPublisher>,
    ) -> Self {
        Self {
            projections,
            store,
            publisher,
        }
    }

    #[instrument(skip_all,fields(delivery_id=%request.delivery_id))]
    pub async fn register_delivery(&self, request: &DeliveryRequest) -> anyhow::Result<bool> {
        self.store
            .register_delivery(&request)
            .await
            .context("failed to register delivery")
    }
    /// Executes one retry for an existing delivery request.
    ///
    /// This method deliberately does not resolve the event's topic
    /// destinations. It retries only the destination in the request.
    pub async fn retry_delivery(&self, request: DeliveryRequest) -> anyhow::Result<()> {
        let destinations = self
            .projections
            .get_destination_details(request.tenant_id, &[request.destination_id])
            .await
            .context("failed to resolve retry destination")?;

        let destination = destinations
            .into_iter()
            .find(|destination| destination.id == request.destination_id);

        let Some(destination) = destination else {
            bail!(
                "destination {} no longer exists in the projection",
                request.destination_id
            );
        };

        if destination.config.status != "active" {
            warn!(
                destination_id = %request.destination_id,
                delivery_id = %request.delivery_id,
                "Retry destination is inactive"
            );
            return Ok(());
        }

        self.execute_delivery(request).await
    }

    /// Manually retries a delivery exactly once.
    ///
    /// Unlike automatic retries, this method never schedules another attempt,
    /// regardless of the configured retry policy.
    pub async fn retry_delivery_once(&self, request: DeliveryRequest) -> anyhow::Result<()> {
        let destinations = self
            .projections
            .get_destination_details(request.tenant_id, &[request.destination_id])
            .await
            .context("failed to resolve retry destination")?;

        let destination = destinations
            .into_iter()
            .find(|destination| destination.id == request.destination_id);

        let Some(destination) = destination else {
            bail!(
                "destination {} no longer exists in the projection",
                request.destination_id
            );
        };

        if destination.config.status != "active" {
            bail!("destination {} is inactive", request.destination_id);
        }

        self.execute_delivery_once(request).await
    }

    #[instrument(skip_all,fields(delivery_id=%request.delivery_id))]
    pub async fn execute_delivery(&self, request: DeliveryRequest) -> anyhow::Result<()> {
        let attempt_id = Uuid::new_v4();

        let acquired = self
            .store
            .acquire_delivery(
                request.tenant_id,
                request.destination_id,
                request.delivery_id,
                attempt_id,
                DELIVERY_LEASE,
            )
            .await
            .context("failed to acquire delivery lease")?;

        let attempt = match acquired {
            AcquireResult::Acquired { attempt } => attempt,

            AcquireResult::Busy => {
                debug!(
                    delivery_id = %request.delivery_id,
                    "Delivery is already being processed"
                );
                return Ok(());
            }

            AcquireResult::Missing => {
                bail!("delivery {} is missing from Redis", request.delivery_id);
            }

            AcquireResult::AlreadySucceeded => {
                debug!(
                    delivery_id = %request.delivery_id,
                    "Delivery has already succeeded"
                );
                return Ok(());
            }

            AcquireResult::AlreadyFailed => {
                debug!(
                    delivery_id = %request.delivery_id,
                    "Delivery has already reached terminal failure"
                );
                return Ok(());
            }

            AcquireResult::InvalidState => {
                bail!("delivery {} has an invalid state", request.delivery_id);
            }
        };

        let mut outcome = self.publisher.deliver(&request, attempt_id, attempt).await;
        let retry_policy = RetryPolicy::new(
            request.destination_config.config.max_attempts,
            request.destination_config.config.retry_delay_ms,
        );

        let next_attempt_at = if outcome.success {
            None
        } else {
            match retry_policy.on_failure(attempt) {
                RetryDecision::RetryAt(next_attempt_at) => {
                    outcome.will_retry = true;
                    Some(next_attempt_at)
                }

                RetryDecision::GiveUp => {
                    outcome.will_retry = false;
                    None
                }
            }
        };

        let completion = self
            .store
            .complete_attempt(
                request.tenant_id,
                request.destination_id,
                &outcome,
                next_attempt_at,
                &request,
            )
            .await
            .context("failed to complete delivery attempt")?;

        match completion {
            CompleteAttemptResult::Completed => {
                info!(
                    delivery_id = %request.delivery_id,
                    attempt,
                    success = outcome.success,
                    will_retry = outcome.will_retry,
                    "Delivery attempt completed"
                );
                Ok(())
            }

            CompleteAttemptResult::Missing => {
                bail!(
                    "delivery {} disappeared before completion",
                    request.delivery_id
                );
            }

            CompleteAttemptResult::NotProcessing => {
                bail!("delivery {} is no longer processing", request.delivery_id);
            }

            CompleteAttemptResult::StaleAttempt => {
                warn!(
                    delivery_id = %request.delivery_id,
                    attempt_id = %attempt_id,
                    "Ignoring completion from a stale delivery attempt"
                );
                Ok(())
            }
        }
    }

    /// Executes exactly one delivery attempt without scheduling retries.
    pub async fn execute_delivery_once(&self, request: DeliveryRequest) -> anyhow::Result<()> {
        let attempt_id = Uuid::new_v4();

        let acquired = self
            .store
            .acquire_delivery(
                request.tenant_id,
                request.destination_id,
                request.delivery_id,
                attempt_id,
                DELIVERY_LEASE,
            )
            .await
            .context("failed to acquire delivery lease")?;

        let attempt = match acquired {
            AcquireResult::Acquired { attempt } => attempt,

            AcquireResult::Busy => {
                debug!(
                    delivery_id = %request.delivery_id,
                    "Delivery is already being processed"
                );
                bail!("delivery is already being processed");
            }

            AcquireResult::Missing => {
                bail!("delivery {} is missing from Redis", request.delivery_id);
            }

            AcquireResult::AlreadySucceeded => {
                bail!("delivery {} has already succeeded", request.delivery_id);
            }

            AcquireResult::AlreadyFailed => {
                bail!(
                    "delivery {} has reached terminal failure",
                    request.delivery_id
                );
            }

            AcquireResult::InvalidState => {
                bail!("delivery {} has an invalid state", request.delivery_id);
            }
        };

        let mut outcome = self.publisher.deliver(&request, attempt_id, attempt).await;

        // Manual retry: never schedule another attempt.
        outcome.will_retry = false;

        let completion = self
            .store
            .complete_attempt(
                request.tenant_id,
                request.destination_id,
                &outcome,
                None, // No next_attempt_at
                &request,
            )
            .await
            .context("failed to complete manual delivery attempt")?;

        match completion {
            CompleteAttemptResult::Completed => {
                info!(
                    delivery_id = %request.delivery_id,
                    attempt,
                    success = outcome.success,
                    "Manual delivery attempt completed"
                );

                Ok(())
            }

            CompleteAttemptResult::Missing => {
                bail!(
                    "delivery {} disappeared before completion",
                    request.delivery_id
                );
            }

            CompleteAttemptResult::NotProcessing => {
                bail!("delivery {} is no longer processing", request.delivery_id);
            }

            CompleteAttemptResult::StaleAttempt => {
                warn!(
                    delivery_id = %request.delivery_id,
                    attempt_id = %attempt_id,
                    "Ignoring completion from a stale manual delivery attempt"
                );

                Ok(())
            }
        }
    }

    /// Derives a stable delivery ID from the event and destination IDs.
    pub fn delivery_id(event_id: Uuid, destination_id: Uuid) -> Uuid {
        let name = format!("{event_id}:{destination_id}");

        Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
    }
}
