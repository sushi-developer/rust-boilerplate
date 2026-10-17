use std::error::Error;
use std::fmt::{self, Display};

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tower_governor::GovernorError;
use tracing::error;

use crate::schemas::default::DefaultResponse;
use crate::traits::default_response::ToDefaultResponse;

const INTERNAL: &str = "Internal server error";

#[derive(Debug)]
pub enum AppError {
    /// Unclassified failure.
    DefaultError,
    /// A required env var is unset or empty.
    MissingConfig(String),
    /// A required env var is set but its value is unusable.
    InvalidConfig {
        variable: String,
        value: String,
    },
    /// Relational database failure.
    Database(sqlx::Error),
    /// Redis command or pool failure.
    Redis(String),
    /// A JSON value (e.g. a cached entry) did not (de)serialize.
    Json(serde_json::Error),
    RateLimitError,
    /// Local-only: a testcontainers container failed to start or report its port.
    Container(String),
    /// Local-only: a schema file under `src/dao/schemas` could not be read.
    Io(String),
}

impl ToDefaultResponse<()> for AppError {
    fn to_default_response(&self) -> (StatusCode, DefaultResponse<()>) {
        let (status, code, message) = match self {
            AppError::DefaultError => (StatusCode::INTERNAL_SERVER_ERROR, "APP-001", INTERNAL),
            AppError::MissingConfig(_) => (StatusCode::INTERNAL_SERVER_ERROR, "APP-002", INTERNAL),
            AppError::InvalidConfig { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, "APP-003", INTERNAL)
            }
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "APP-004", INTERNAL),
            AppError::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "APP-005", INTERNAL),
            AppError::Json(_) => (StatusCode::INTERNAL_SERVER_ERROR, "APP-006", INTERNAL),
            AppError::RateLimitError => (
                StatusCode::TOO_MANY_REQUESTS,
                "APP-010",
                "Too many requests",
            ),
            AppError::Container(_) => (StatusCode::INTERNAL_SERVER_ERROR, "APP-011", INTERNAL),
            AppError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "APP-013", INTERNAL),
        };

        (status, DefaultResponse::error(code, message))
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DefaultError => write!(f, "Internal error"),
            AppError::MissingConfig(variable) => write!(f, "Config: {variable} is not set"),
            AppError::InvalidConfig { variable, value } => write!(f, "Config: {variable}: {value}"),
            AppError::Database(e) => write!(f, "SQL error: {e}"),
            AppError::Redis(msg) => write!(f, "Redis error: {msg}"),
            AppError::Json(e) => write!(f, "JSON error: {e}"),
            AppError::RateLimitError => write!(f, "Too many requests"),
            AppError::Container(msg) => write!(f, "Container error: {msg}"),
            AppError::Io(msg) => write!(f, "IO error: {msg}"),
        }
    }
}

impl Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = self.to_default_response();
        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        error!("{}", error);
        AppError::Database(error)
    }
}

impl From<deadpool_redis::redis::RedisError> for AppError {
    fn from(error: deadpool_redis::redis::RedisError) -> Self {
        error!("{}", error);
        AppError::Redis(error.to_string())
    }
}

impl From<deadpool_redis::PoolError> for AppError {
    fn from(error: deadpool_redis::PoolError) -> Self {
        error!("{}", error);
        AppError::Redis(error.to_string())
    }
}

impl From<deadpool_redis::CreatePoolError> for AppError {
    fn from(error: deadpool_redis::CreatePoolError) -> Self {
        error!("{}", error);
        AppError::Redis(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        error!("{}", error);
        AppError::Json(error)
    }
}

impl From<GovernorError> for AppError {
    fn from(error: GovernorError) -> Self {
        error!("{}", error);
        match error {
            GovernorError::TooManyRequests { .. } => AppError::RateLimitError,
            _ => AppError::DefaultError,
        }
    }
}

impl From<testcontainers::TestcontainersError> for AppError {
    fn from(error: testcontainers::TestcontainersError) -> Self {
        error!("{}", error);
        AppError::Container(error.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        error!("{}", error);
        AppError::Io(error.to_string())
    }
}
