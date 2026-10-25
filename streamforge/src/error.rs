use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("connection error: {0}")]
    Connection(String),

    #[error("publish failed: {0}")]
    Publish(String),

    #[error("consume failed: {0}")]
    Consume(String),

    #[error("acknowledgement failed: {0}")]
    Ack(String),

    #[error("queue setup failed: {0}")]
    Setup(String),

    #[error("invalid message: {0}")]
    InvalidMessage(String),

    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

pub type Result<T> = std::result::Result<T, QueueError>;
