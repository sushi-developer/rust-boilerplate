use axum::http::StatusCode;

use crate::schemas::default::DefaultResponse;

/// The single conversion every error type in `errors/` implements.
pub trait ToDefaultResponse<T> {
    fn to_default_response(&self) -> (StatusCode, DefaultResponse<T>);
}
