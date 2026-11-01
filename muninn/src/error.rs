use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// The lease expired and another worker took the row.
    #[error("inbox message {0} is no longer claimed by this worker")]
    LeaseLost(i64),

    #[error("queue error: {0}")]
    Queue(String),

    #[error("stream source error: {0}")]
    Source(String),
}
