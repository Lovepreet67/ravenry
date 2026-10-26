use iam_contracts::auth::error::AuthError::{self, BadRequest, Unauthorized};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PublishError {
    #[error("topic not found")]
    TopicNotFound,

    #[error("publishing is not permitted")]
    Forbidden,

    #[error("invalid publish request: {0}")]
    InvalidRequest(String),

    #[error("topic projection is unavailable")]
    ProjectionUnavailable,

    #[error("message queue is unavailable")]
    QueueUnavailable,
}
