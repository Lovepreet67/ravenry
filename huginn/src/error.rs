use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use iam_contracts::auth::error::AuthError;
use serde_json::json;
use tracing::error;

use crate::domain::pagination::InvalidCursor;

pub type AppResult<T> = Result<T, AppError>;

/// Storage-agnostic. Each backend maps its own errors into this.
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("storage backend error: {0}")]
    Backend(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Auth(#[from] AuthError),

    /// Also returned for rows owned by another tenant, so IDs can't be probed.
    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error(transparent)]
    Repo(#[from] RepoError),

    #[error("upstream service error: {0}")]
    Upstream(String),
}

impl From<InvalidCursor> for AppError {
    fn from(e: InvalidCursor) -> Self {
        Self::BadRequest(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            // Reuse the auth module's own mapping (401/403).
            AppError::Auth(e) => return e.into_response(),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::Upstream(m) => {
                error!(error = %m, "upstream failure");
                (
                    StatusCode::BAD_GATEWAY,
                    "upstream service unavailable".into(),
                )
            }
            AppError::Repo(e) => {
                // Never leak query or connection details to clients.
                error!(error = %e, "repository failure");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
