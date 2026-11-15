use thiserror::Error;

#[derive(Debug, Error)]
pub enum RetryError {
    #[error("delivery not found")]
    DeliveryNotFound,

    #[error("invalid publish request: {0}")]
    InvalidRequest(String),

    #[error("publishing is not permitted")]
    Forbidden,

    #[error("message queue is unavailable")]
    QueueUnavailable,
}
