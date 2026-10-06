// core/src/queue/redis_queue.rs
use super::{Message, Queue, QueueError};
use async_trait::async_trait;
use redis::{AsyncCommands, RedisError, aio::ConnectionManager};

pub struct RedisQueue {
    conn: ConnectionManager,
}

impl RedisQueue {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }
}

impl From<RedisError> for QueueError {
    fn from(e: RedisError) -> Self {
        QueueError::Connection(e.to_string())
    }
}

#[async_trait]
impl Queue for RedisQueue {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<String, QueueError> {
        let mut conn = self.conn.clone();
        let id: String = redis::cmd("XADD")
            .arg(topic)
            .arg("*") // let Redis auto-generate the entry id
            .arg("payload")
            .arg(payload)
            .query_async(&mut conn)
            .await
            .map_err(|e| QueueError::Publish(e.to_string()))?;
        Ok(id)
    }

    async fn ensure_group(&self, topic: &str, group: &str) -> Result<(), QueueError> {
        let mut conn = self.conn.clone();
        let result: Result<String, RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(topic)
            .arg(group)
            .arg("$") // start the group at "now" — only new messages
            .arg("MKSTREAM") // create the stream itself if it doesn't exist yet
            .query_async(&mut conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            // BUSYGROUP means the group already exists — that's fine, not an error
            Err(e) if e.to_string().contains("BUSYGROUP") => Ok(()),
            Err(e) => Err(QueueError::Setup(e.to_string())),
        }
    }

    async fn consume(
        &self,
        topic: &str,
        group: &str,
        consumer: &str,
        count: usize,
        block_ms: usize,
    ) -> Result<Vec<Message>, QueueError> {
        let mut conn = self.conn.clone();

        // ">" means "give me only messages never delivered to any consumer in this group"
        let reply: redis::streams::StreamReadReply = conn
            .xread_options(
                &[topic],
                &[">"],
                &redis::streams::StreamReadOptions::default()
                    .group(group, consumer)
                    .count(count)
                    .block(block_ms),
            )
            .await
            .map_err(|e| QueueError::Consume(e.to_string()))?;

        let mut messages = Vec::new();
        for stream_key in reply.keys {
            for entry in stream_key.ids {
                if let Some(redis::Value::BulkString(bytes)) = entry.map.get("payload") {
                    messages.push(Message {
                        id: entry.id,
                        payload: bytes.clone(),
                    });
                }
            }
        }
        Ok(messages)
    }

    async fn ack(&self, topic: &str, group: &str, message_id: &str) -> Result<(), QueueError> {
        let mut conn = self.conn.clone();
        let _: i64 = conn
            .xack(topic, group, &[message_id])
            .await
            .map_err(|e| QueueError::Ack(e.to_string()))?;
        Ok(())
    }
}
