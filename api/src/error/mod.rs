use std::format;

use axum::{http::StatusCode, response::IntoResponse};
use redis::RedisError;
use serde::{Deserialize, Serialize};

use crate::Error::ApiError::{Gen, Generic};

#[derive(Serialize, Deserialize, Debug)]
pub enum ApiError {
    Generic,
    Gen(&'static str),
    RedisError(String),
    Argon(String),
    Sqlx(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl From<RedisError> for ApiError {
    fn from(value: RedisError) -> Self {
        Self::RedisError(format!("{}", value))
    }
}
impl From<argon2::password_hash::Error> for ApiError {
    fn from(value: argon2::password_hash::Error) -> Self {
        Self::Argon(value.to_string())
    }
}
impl From<argon2::password_hash::phc::Error> for ApiError {
    fn from(value: argon2::password_hash::phc::Error) -> Self {
        Self::Argon(value.to_string())
    }
}
impl From<sqlx::Error> for ApiError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Generic => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response(),
            Gen(val) => (StatusCode::INTERNAL_SERVER_ERROR, val).into_response(),
            Self::RedisError(val) => (StatusCode::INTERNAL_SERVER_ERROR, val).into_response(),
            Self::Argon(val) => (StatusCode::INTERNAL_SERVER_ERROR, val).into_response(),
            Self::Sqlx(val) => (StatusCode::INTERNAL_SERVER_ERROR, val).into_response(),
        }
    }
}
