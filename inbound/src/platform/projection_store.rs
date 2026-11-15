use anyhow::Context;
use iam_contracts::keys;
use redis::{aio::ConnectionManager, AsyncCommands};
use uuid::Uuid;

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

    pub async fn resolve_topic_id(
        &self,
        tenant_id: Uuid,
        topic_path: &str,
    ) -> anyhow::Result<Option<Uuid>> {
        let mut connection = self.connection.clone();

        let topic_id: Option<String> = connection
            .hget(keys::tenant_topics(tenant_id), topic_path)
            .await?;

        topic_id
            .map(|id| {
                Uuid::parse_str(&id)
                    .map_err(|err| anyhow::anyhow!("invalid topic UUID in Redis projection: {err}"))
            })
            .transpose()
    }
    pub async fn assert_delivery_exist(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        delivery_id: Uuid,
    ) -> anyhow::Result<i32> {
        let mut connection = self.connection.clone();
        let key = keys::delivery_state(tenant_id, destination_id, delivery_id);

        let (status, attempts): (Option<String>, Option<i32>) =
            connection.hmget(&key, &["status", "attempts"]).await?;

        match (status, attempts) {
            (Some(status), Some(attempts)) if status == "failed" => Ok(attempts),
            (Some(_), _) => Err(anyhow::anyhow!("Only failed deliveries can be retried")),
            (None, _) => Err(anyhow::anyhow!("Invalid delivery id")),
        }
    }
    pub async fn mark_delivery_pending(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        delivery_id: Uuid,
    ) -> anyhow::Result<()> {
        let mut connection = self.connection.clone();
        let key = keys::delivery_state(tenant_id, destination_id, delivery_id);
        connection
            .hset::<_, _, _, ()>(&key, "status", "pending")
            .await
            .context("Error while updating the status")?;
        return Ok(());
    }
}
