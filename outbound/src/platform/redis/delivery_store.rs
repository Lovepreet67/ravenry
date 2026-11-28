use std::time::Duration;

use anyhow::Context;
use iam_contracts::keys::ACTIVE_DESTINATION_STREAMS;
use redis::{
    aio::{ConnectionManager, ConnectionManagerConfig},
    AsyncCommands, Pipeline, Script,
};
use streamforge::QueueError;
use tracing::instrument;
use uuid::Uuid;

use crate::application::delivery::model::DeliveryRequest;

const SET_TO_QUEUE: &str = include_str!("scripts/set_to_queue.lua");

#[derive(Clone)]
pub struct DeliveryStore {
    connection: ConnectionManager,
}

impl DeliveryStore {
    pub async fn connect(redis_url: &str) -> anyhow::Result<Self> {
        let client =
            redis::Client::open(redis_url).map_err(|e| QueueError::Connection(e.to_string()))?;
        let cfg = ConnectionManagerConfig::new()
            .set_connection_timeout(Some(Duration::from_secs(5)))
            .set_response_timeout(Some(Duration::from_secs(10)));
        let connection = ConnectionManager::new_with_config(client, cfg)
            .await
            .map_err(|e| QueueError::Connection(e.to_string()))?;

        Ok(Self { connection })
    }

    pub async fn health_check(&self) -> anyhow::Result<()> {
        let mut connection = self.connection.clone();

        redis::cmd("PING")
            .query_async::<String>(&mut connection)
            .await
            .context("Redis health check failed")?;

        Ok(())
    }

    /// Adds a retry DeliveryRequest to the destination retry ZSET scored by `score_ms` (epoch millis).
    pub async fn schedule_retry(
        &self,
        retry_key: &str,
        retry_request: &DeliveryRequest,
        score_ms: i64,
    ) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(retry_request)
            .context("failed to serialize retry DeliveryRequest")?;

        let mut conn = self.connection.clone();
        redis::cmd("ZADD")
            .arg(retry_key)
            .arg(score_ms)
            .arg(payload)
            .query_async::<()>(&mut conn)
            .await
            .with_context(|| format!("failed to ZADD retry entry into {retry_key}"))?;

        Ok(())
    }

    /// Atomically promotes due items from `retry_zset_key` into `delivery_stream_key` (`XADD`).
    #[instrument(skip(self))]
    pub async fn promote_due_retries(
        &self,
        retry_zset_key: &str,
        delivery_stream_key: &str,
        now_ms: i64,
        batch_limit: usize,
    ) -> anyhow::Result<usize> {
        let mut conn = self.connection.clone();
        let script = Script::new(SET_TO_QUEUE);

        let moved: usize = script
            .key(retry_zset_key)
            .key(delivery_stream_key)
            .arg(now_ms)
            .arg(batch_limit)
            .invoke_async(&mut conn)
            .await
            .with_context(|| {
                format!(
                    "failed to atomically promote retries from {retry_zset_key} to {delivery_stream_key}"
                )
            })?;
        Ok(moved)
    }
    pub async fn mark_destinations_active(
        &self,
        destinations: &[(Uuid, Uuid)],
    ) -> anyhow::Result<()> {
        if destinations.is_empty() {
            return Ok(());
        }

        let mut pipe = Pipeline::new();
        for (tenant_id, destination_id) in destinations {
            // Adjust key helper if you have one in `iam_contracts::keys`
            let val: String = format!("{}_{}", tenant_id, destination_id);
            pipe.sadd(ACTIVE_DESTINATION_STREAMS, val).ignore();
        }

        let mut conn = self.connection.clone();
        pipe.query_async::<()>(&mut conn)
            .await
            .context("failed to mark destinations active in Redis")?;

        Ok(())
    }
    pub async fn mark_destinations_inactive(
        &self,
        destinations: &[(Uuid, Uuid)],
    ) -> anyhow::Result<()> {
        if destinations.is_empty() {
            return Ok(());
        }

        let mut pipe = Pipeline::new();
        for (tenant_id, destination_id) in destinations {
            let val: String = format!("{}_{}", tenant_id, destination_id);
            pipe.srem(ACTIVE_DESTINATION_STREAMS, val).ignore();
        }

        let mut conn = self.connection.clone();
        pipe.query_async::<()>(&mut conn)
            .await
            .context("failed to mark destinations inactive in Redis")?;

        Ok(())
    }
    pub async fn list_active_destinations(&self) -> anyhow::Result<Vec<(Uuid, Uuid)>> {
        let mut conn = self.connection.clone();
        let raw_members: Vec<String> = conn
            .smembers(ACTIVE_DESTINATION_STREAMS)
            .await
            .context("failed to list active destinations from Redis")?;

        let mut destinations = Vec::with_capacity(raw_members.len());
        for member in raw_members {
            let (tenant_str, dest_str) = member.split_once('_').ok_or_else(|| {
                anyhow::anyhow!("invalid active destination member format: '{member}'")
            })?;

            let tenant_id = Uuid::parse_str(tenant_str).with_context(|| {
                format!("invalid tenant_id UUID in active destination '{member}'")
            })?;
            let destination_id = Uuid::parse_str(dest_str).with_context(|| {
                format!("invalid destination_id UUID in active destination '{member}'")
            })?;

            destinations.push((tenant_id, destination_id));
        }

        Ok(destinations)
    }
}
