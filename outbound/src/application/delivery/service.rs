use anyhow::{Context, Result};
use iam_contracts::events::PublishedEvent;
use tracing::info;

use crate::{
    application::delivery::http_publisher::{DeliveryOutcome, HttpPublisher},
    model::ResolvedDestination,
    platform::projection_store::ProjectionStore,
};

#[derive(Clone)]
pub struct DeliveryService {
    projections: ProjectionStore,
    publisher: HttpPublisher,
}

impl DeliveryService {
    pub fn new(projections: ProjectionStore, publisher: HttpPublisher) -> Self {
        Self {
            projections,
            publisher,
        }
    }

    pub async fn deliver_event(&self, event: &PublishedEvent) -> Result<Vec<DeliveryOutcome>> {
        let destinations = self
            .projections
            .resolve_topic_destinations(event.tenant_id, event.topic_id)
            .await
            .context("failed to resolve topic destinations")?;

        let mut outcomes = Vec::with_capacity(destinations.len());
        info!("Destinations counts: {}", destinations.len());

        for destination in destinations {
            if destination.config.status != "active" {
                continue;
            }

            let outcome = self
                .publisher
                .deliver(
                    &destination,
                    event.event_id,
                    event.tenant_id,
                    event.topic_id,
                    &event.payload,
                )
                .await;

            outcomes.push(outcome);
        }

        Ok(outcomes)
    }
}
