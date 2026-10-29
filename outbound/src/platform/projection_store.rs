use std::collections::HashMap;

use anyhow::{bail, Context};
use iam_contracts::keys;
use redis::{aio::ConnectionManager, AsyncCommands};
use uuid::Uuid;

use crate::model::{DestinationProjection, ResolvedDestination};

#[derive(Clone)]
pub struct ProjectionStore {
    connection: ConnectionManager,
}

impl ProjectionStore {
    pub async fn connect(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = ConnectionManager::new(client).await?;

        Ok(Self { connection })
    }
    pub async fn health_check(&self) -> anyhow::Result<()> {
        let mut connection = self.connection.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut connection)
            .await?;
        Ok(())
    }

    pub async fn resolve_topic_destinations(
        &self,
        tenant_id: Uuid,
        topic_id: Uuid,
    ) -> anyhow::Result<Vec<ResolvedDestination>> {
        let mut connection = self.connection.clone();

        // Round trip 1: fetch every subscribed destination ID.
        let raw_ids: Vec<String> = connection
            .smembers(keys::topic_subscriptions(tenant_id, topic_id))
            .await?;

        if raw_ids.is_empty() {
            return Ok(Vec::new());
        }

        let destination_ids: Vec<Uuid> = raw_ids
            .into_iter()
            .map(|raw_id| {
                Uuid::parse_str(&raw_id).with_context(|| {
                    format!(
                        "invalid destination UUID in subscription set \
                     (tenant_id={tenant_id}, topic_id={topic_id}, id={raw_id})"
                    )
                })
            })
            .collect::<anyhow::Result<_>>()?;

        // Round trip 2: fetch all destination hashes in one pipeline.
        let mut pipeline = redis::pipe();

        for destination_id in &destination_ids {
            pipeline
                .cmd("HGETALL")
                .arg(keys::destination(tenant_id, *destination_id));
        }

        let configs: Vec<HashMap<String, String>> = pipeline.query_async(&mut connection).await?;

        let mut destinations = Vec::with_capacity(destination_ids.len());

        for (destination_id, fields) in destination_ids.into_iter().zip(configs) {
            if fields.is_empty() {
                bail!(
                    "destination projection missing \
                 (tenant_id={tenant_id}, destination_id={destination_id})"
                );
            }

            let config: DestinationProjection =
                serde_json::from_value(serde_json::to_value(fields)?).with_context(|| {
                    format!(
                        "invalid destination projection \
                         (tenant_id={tenant_id}, destination_id={destination_id})"
                    )
                })?;

            destinations.push(ResolvedDestination {
                id: destination_id,
                config,
            });
        }

        Ok(destinations)
    }
}
