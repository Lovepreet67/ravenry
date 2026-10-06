// core/src/queue/mod.rs
use async_trait::async_trait;
use thiserror::Error;
pub mod redis_stream;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String, // queue-assigned id (e.g. Redis stream entry id, Kafka offset)
    pub payload: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("connection error: {0}")]
    Connection(String),
    #[error("publish failed: {0}")]
    Publish(String),
    #[error("consume failed: {0}")]
    Consume(String),
    #[error("ack failed: {0}")]
    Ack(String),
    #[error("setup failed: {0}")]
    Setup(String),
}

#[async_trait]
pub trait Queue: Send + Sync {
    /// Publish a message to a topic. Returns the queue-assigned message id.
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<String, QueueError>;

    /// Ensure a consumer group exists on a topic. Idempotent — safe to call every startup.
    async fn ensure_group(&self, topic: &str, group: &str) -> Result<(), QueueError>;

    /// Pull up to `count` undelivered messages for this consumer group.
    /// `block_ms` = 0 means return immediately; >0 means wait up to that long for new messages.
    async fn consume(
        &self,
        topic: &str,
        group: &str,
        consumer: &str,
        count: usize,
        block_ms: usize,
    ) -> Result<Vec<Message>, QueueError>;

    /// Acknowledge successful processing — removes the message from the group's pending list.
    async fn ack(&self, topic: &str, group: &str, message_id: &str) -> Result<(), QueueError>;
}
