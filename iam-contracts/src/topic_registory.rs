use std::time::Duration;

use crate::{
    keys::ACTIVE_TOPIC_STREAMS,
    streams::{topic_events, topic_events_dlq},
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use redis::{
    AsyncCommands,
    aio::{ConnectionManager, ConnectionManagerConfig},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TopicStream {
    pub tenant_id: Uuid,
    pub topic_id: Uuid,
}

impl TopicStream {
    pub fn topic_events(&self) -> String {
        topic_events(self.tenant_id, self.topic_id)
    }
    pub fn topic_dlq(&self) -> String {
        topic_events_dlq(self.tenant_id, self.topic_id)
    }
}

#[async_trait]
pub trait TopicRegistry: Send + Sync {
    async fn list_active(&self) -> Result<Vec<TopicStream>>;
}

#[derive(Clone)]
pub struct RedisTopicRegistry {
    connection: ConnectionManager,
}

impl RedisTopicRegistry {
    pub async fn connect(redis_url: &str) -> Result<Self> {
        let client =
            redis::Client::open(redis_url).context("Error while creating the redis client")?;
        let cfg = ConnectionManagerConfig::new()
            .set_connection_timeout(Some(Duration::from_secs(5)))
            .set_response_timeout(Some(Duration::from_secs(10)));
        let connection = ConnectionManager::new_with_config(client, cfg)
            .await
            .context("Error while creating the connection")?;

        Ok(Self { connection })
    }
}

#[async_trait]
impl TopicRegistry for RedisTopicRegistry {
    async fn list_active(&self) -> Result<Vec<TopicStream>> {
        let mut connection = self.connection.clone();

        let entries: Vec<String> = connection
            .smembers(ACTIVE_TOPIC_STREAMS)
            .await
            .context("failed to read active topic registry")?;

        let mut topics = Vec::with_capacity(entries.len());

        for entry in entries {
            let (tenant_id, topic_id) = entry
                .split_once(':')
                .with_context(|| format!("invalid topic registry entry: {entry}"))?;

            let tenant_id = Uuid::parse_str(tenant_id)
                .with_context(|| format!("invalid tenant UUID in registry entry: {entry}"))?;

            let topic_id = Uuid::parse_str(topic_id)
                .with_context(|| format!("invalid topic UUID in registry entry: {entry}"))?;

            topics.push(TopicStream {
                tenant_id,
                topic_id,
            });
        }

        topics.sort_by_key(|topic| (topic.tenant_id, topic.topic_id));

        Ok(topics)
    }
}
