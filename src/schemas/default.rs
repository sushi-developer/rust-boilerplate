// Shared response helpers for upcoming handlers; unused until the first feature lands.
#![allow(dead_code)]
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct DefaultResponse<T> {
    /// `"success"` or `"error"`.
    pub status: &'static str,
    /// `"200"` on success, a stable app code such as `APP-004` on error.
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

impl DefaultResponse<()> {
    /// `data` is always `null` on the error arm — the client never has to check.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: "error",
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }
}

pub fn default_response<T>(t: T) -> Json<DefaultResponse<T>> {
    message_response(t, "Success")
}

pub fn message_response<T>(t: T, message: impl Into<String>) -> Json<DefaultResponse<T>> {
    Json(DefaultResponse {
        status: "success",
        code: String::from("200"),
        message: message.into(),
        data: Some(t),
    })
}

/// Paginated payload that reports the full `total`; the client derives "has next page"
/// from `offset + result.len() < total`.
#[derive(Serialize, ToSchema)]
pub struct TotalPagination<T> {
    pub result: Vec<T>,
    pub total: i64,
}

/// Documentation-only mirror of the error shape produced by `AppError`'s
/// `IntoResponse` (`DefaultResponse<()>`, i.e. `data: null`).
#[derive(Serialize, ToSchema)]
pub struct ErrorRes {
    /// Always `"error"`.
    pub status: String,
    /// Stable app code, e.g. `APP-004`.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

pub fn default_offset() -> i64 {
    0
}

pub fn default_limit() -> i64 {
    10
}
