use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::application::publish::error::PublishError;
use iam_contracts::auth::error::AuthError;

pub enum HttpError {
    Auth(AuthError),
    Publish(PublishError),
}

impl From<AuthError> for HttpError {
    fn from(err: AuthError) -> Self {
        Self::Auth(err)
    }
}

impl From<PublishError> for HttpError {
    fn from(err: PublishError) -> Self {
        Self::Publish(err)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        match self {
            Self::Auth(err) => err.into_response(),

            Self::Publish(err) => {
                let (status, message) = match err {
                    PublishError::TopicNotFound => (StatusCode::NOT_FOUND, "topic not found"),
                    PublishError::Forbidden => {
                        (StatusCode::FORBIDDEN, "publishing is not permitted")
                    }
                    PublishError::InvalidRequest(_) => {
                        (StatusCode::BAD_REQUEST, "invalid publish request")
                    }
                    PublishError::ProjectionUnavailable | PublishError::QueueUnavailable => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "publishing service temporarily unavailable",
                    ),
                };

                (status, Json(json!({ "error": message }))).into_response()
            }
        }
    }
}
