use std::{sync::Arc, time::Duration};

use anyhow::{bail, Context};
use iam_contracts::events::PublishedEvent;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    application::delivery::{
        http_publisher::HttpPublisher,
        model::{AcquireResult, CompleteAttemptResult, DeliveryRequest},
        retry::{RetryDecision, RetryPolicy},
    },
    model::ResolvedDestination,
    platform::projection_store::ProjectionStore,
    platform::redis::delivery_store::DeliveryStore,
};

const DELIVERY_LEASE: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct DeliveryService {
    projections: Arc<ProjectionStore>,
    store: Arc<DeliveryStore>,
    publisher: Arc<HttpPublisher>,
    retry_policy: Arc<RetryPolicy>,
}

impl DeliveryService {
    pub fn new(
        projections: Arc<ProjectionStore>,
        store: Arc<DeliveryStore>,
        publisher: Arc<HttpPublisher>,
        retry_policy: Arc<RetryPolicy>,
    ) -> Self {
        Self {
            projections,
            store,
            publisher,
            retry_policy,
        }
    }

    /// Resolves destinations for an event and registers each delivery.
    ///
    /// Each event/destination pair gets a deterministic delivery ID.
    /// Reprocessing the same event therefore cannot create a new logical
    /// delivery for the same destination.
    pub async fn deliver_event(&self, event: &PublishedEvent) -> anyhow::Result<()> {
        let destinations = if event.destinations.is_empty() {
            self.projections
                .resolve_topic_destinations(event.tenant_id, event.topic_id)
                .await
                .context("failed to resolve topic destinations")?
        } else {
            self.projections
                .get_destination_details(event.tenant_id, &event.destinations)
                .await
                .context("failed to resolve the listed destinations")?
        };

        info!(
            event_id = %event.event_id,
            destination_count = destinations.len(),
            "Resolved event destinations"
        );

        for destination in destinations {
            if destination.config.status != "active" {
                debug!(
                    destination_id = %destination.id,
                    "Skipping inactive destination"
                );
                continue;
            }

            let request = DeliveryRequest {
                delivery_id: Self::delivery_id(event.event_id, destination.id),
                event_id: event.event_id,
                tenant_id: event.tenant_id,
                topic_id: event.topic_id,
                destination_id: destination.id,
                payload: event.payload.clone(),
            };

            let registered = self
                .store
                .register_delivery(&request)
                .await
                .context("failed to register delivery")?;

            if !registered {
                debug!(
                    delivery_id = %request.delivery_id,
                    "Delivery already registered; checking whether it can be acquired"
                );
            }

            self.execute_delivery(request, destination).await?;
        }

        Ok(())
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

        self.execute_delivery(request, destination).await
    }

    async fn execute_delivery(
        &self,
        request: DeliveryRequest,
        destination: ResolvedDestination,
    ) -> anyhow::Result<()> {
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

        let mut outcome = self
            .publisher
            .deliver(&destination, &request, attempt_id, attempt)
            .await;

        let next_attempt_at = if outcome.success {
            None
        } else {
            match self.retry_policy.on_failure(attempt) {
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

    /// Derives a stable delivery ID from the event and destination IDs.
    fn delivery_id(event_id: Uuid, destination_id: Uuid) -> Uuid {
        let name = format!("{event_id}:{destination_id}");

        Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
    }
}
