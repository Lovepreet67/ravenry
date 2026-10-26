use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use iam_contracts::auth::error::AuthError;
use serde::Serialize;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Resource not found")]
    NotFound,

    #[error("{0}")]
    Conflict(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Internal server error")]
    Internal(#[source] anyhow::Error),

    #[error("Not handled")]
    NotHandled(#[source] anyhow::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => ApiError::NotFound,
            sqlx::Error::Database(db_err) => {
                // Postgres unique_violation = SQLSTATE 23505
                if db_err.code().as_deref() == Some("23505") {
                    ApiError::Conflict(db_err.message().to_string())
                } else {
                    ApiError::Internal(anyhow::anyhow!(err))
                }
            }
            _ => ApiError::Internal(anyhow::anyhow!(err)),
        }
    }
}
impl From<validator::ValidationErrors> for ApiError {
    fn from(err: validator::ValidationErrors) -> Self {
        let message = err
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                let reasons: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        e.message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("invalid value for '{field}'"))
                    })
                    .collect();
                format!("{field}: {}", reasons.join(", "))
            })
            .collect::<Vec<_>>()
            .join("; ");

        ApiError::Validation(message)
    }
}

// temp implementation needs modification
impl From<uuid::Error> for ApiError {
    fn from(value: uuid::Error) -> Self {
        return ApiError::NotHandled(anyhow::anyhow!(value));
    }
}

impl From<argon2::password_hash::phc::Error> for ApiError {
    fn from(value: argon2::password_hash::phc::Error) -> Self {
        return ApiError::NotHandled(anyhow::anyhow!(value));
    }
}
impl From<AuthError> for ApiError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::BadRequest(msg) => Self::BadRequest(msg),
            AuthError::Unauthorized => Self::Unauthorized,
            AuthError::Forbidden => Self::Forbidden,
            AuthError::Internal(err) => Self::Internal(err),
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_owned()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_owned()),
            Self::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_owned()),
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            Self::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            Self::Internal(err) => {
                // Replace with tracing::error! in production.
                eprintln!("Internal error: {err:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
            Self::NotHandled(err) => {
                // Replace with tracing::error! in production.
                tracing::error!("Not handlerd error: {err:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}
