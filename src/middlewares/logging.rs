use std::sync::LazyLock;

use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{
        HeaderMap,
        header::{CONTENT_LENGTH, CONTENT_TYPE},
    },
    middleware::Next,
    response::Response,
};
use regex::Regex;
use tracing::{debug, warn};

use crate::AppState;
use crate::config::Environment;

/// Cap on how much of a request/response body is buffered for logging.
const MAX_LOG_BODY_BYTES: usize = 64 * 1024;

/// Cap on how much of a buffered body is echoed into a log line.
const MAX_LOGGED_BODY_CHARS: usize = 2048;

/// Stands in for a body that was never buffered (multipart, binary, oversized, or chunked
/// with no declared length).
const NOT_BUFFERED: &str = "<not buffered>";

/// Values of these keys never reach the log — OTPs and tokens are credentials, and a
/// failed request is not a reason to leak them into production logs.
static SENSITIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)("(?:otp|password|token|access_token|refresh_token|secret|authorization)"\s*:\s*)("(?:[^"\\]|\\.)*"|[^,}\s]+)"#,
    )
    .expect("SENSITIVE regex is valid")
});

/// Render a body for logging: redact credential-shaped fields, then truncate on a char
/// boundary. Safe on malformed JSON and on non-UTF-8 bytes — the bodies worth logging are
/// the broken ones, so this is a regex over raw text rather than a `serde_json::Value` walk.
fn sanitize_body(bytes: &[u8]) -> String {
    let raw = String::from_utf8_lossy(bytes);
    let redacted = SENSITIVE.replace_all(&raw, r#"${1}"[redacted]""#);

    if redacted.chars().count() <= MAX_LOGGED_BODY_CHARS {
        return redacted.into_owned();
    }
    let head: String = redacted.chars().take(MAX_LOGGED_BODY_CHARS).collect();
    format!("{head}…(truncated, {} bytes total)", bytes.len())
}

/// Text we can render as UTF-8. Multipart and binary uploads are excluded.
fn is_text_content_type(headers: &HeaderMap) -> bool {
    match headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
        Some(ct) => {
            ct.starts_with("application/json")
                || ct.starts_with("text/")
                || ct.starts_with("application/x-www-form-urlencoded")
        }
        None => false,
    }
}

/// Whether to buffer a *request* body. Deliberately strict: an explicit `Content-Length`
/// within the cap is required, because `to_bytes` consumes the body and a failure to buffer
/// would destroy the request. Chunked/undeclared, oversized, multipart and binary bodies are
/// forwarded untouched and simply never logged.
fn is_loggable_request_body(headers: &HeaderMap) -> bool {
    let Some(len) = headers
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
    else {
        return false;
    };
    len > 0 && len <= MAX_LOG_BODY_BYTES && is_text_content_type(headers)
}

/// Whether to buffer a *response* body. Looser than the request side — axum does not set
/// `Content-Length` on responses (hyper adds it at serialization), so requiring it would
/// never log anything.
fn is_loggable_response_body(headers: &HeaderMap) -> bool {
    if let Some(len) = headers
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        && len > MAX_LOG_BODY_BYTES
    {
        return false;
    }
    match headers.get(CONTENT_TYPE) {
        Some(_) => is_text_content_type(headers),
        None => true, // no / empty body
    }
}

/// Two loggers over one buffered body:
/// - `local` only, at `debug!` — `request` / `response` for every call, success included.
/// - every environment, at `warn!` — `request` / `request failed` for every 4xx/5xx.
///
/// A successful request outside `local` logs nothing and never buffers its response.
pub async fn log_request_response(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let is_local = state.config.environment == Environment::Local;

    let (parts, body) = req.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri.clone();

    // Buffered in every environment, not just local: the payload behind a 4xx/5xx is the
    // point, and the status is only known after `next.run`. `Bytes` clones are refcount
    // bumps, so the handler still receives the same buffer.
    let (req, buffered) = if is_loggable_request_body(&parts.headers) {
        match to_bytes(body, MAX_LOG_BODY_BYTES).await {
            Ok(bytes) => {
                if is_local {
                    debug!(%method, %uri, body = %sanitize_body(&bytes), "request");
                }
                let req = Request::from_parts(parts, Body::from(bytes.clone()));
                (req, Some(bytes))
            }
            Err(err) => {
                // Unreachable now that buffering requires a Content-Length within the cap;
                // if it happens the body is already consumed, so forward empty rather than fail.
                warn!(%method, %uri, error = %err, "failed to buffer request body for logging");
                (Request::from_parts(parts, Body::empty()), None)
            }
        }
    } else {
        if is_local {
            debug!(%method, %uri, "request (body not logged)");
        }
        (Request::from_parts(parts, body), None)
    };

    let res = next.run(req).await;

    let status = res.status();
    let failed = status.is_client_error() || status.is_server_error();

    // Nobody is reading this one: it succeeded and we are not tracing locally. Hand the
    // response back untouched rather than buffering it.
    if !failed && !is_local {
        return res;
    }

    if failed {
        // Re-emitted at `warn!` rather than reusing the `debug!` above: production runs
        // `TRACE_LEVEL=info`, which drops `DEBUG` entirely. This is the request line prod sees.
        // On `local` it does double up with the `debug!` — that one is pre-handler evidence,
        // this one is part of a self-contained failure record.
        match buffered.as_deref() {
            Some(bytes) => warn!(%method, %uri, body = %sanitize_body(bytes), "request"),
            None => warn!(%method, %uri, body = NOT_BUFFERED, "request"),
        }
    }

    let (parts, body) = res.into_parts();
    if !is_loggable_response_body(&parts.headers) {
        if failed {
            warn!(
                %method, %uri, status = %parts.status,
                "request failed (response body not logged)"
            );
        } else {
            debug!(status = %parts.status, "response (body not logged)");
        }
        return Response::from_parts(parts, body);
    }

    match to_bytes(body, MAX_LOG_BODY_BYTES).await {
        Ok(bytes) => {
            if failed {
                warn!(
                    %method, %uri, status = %parts.status,
                    response_body = %sanitize_body(&bytes),
                    "request failed"
                );
            } else {
                debug!(status = %parts.status, body = %sanitize_body(&bytes), "response");
            }
            Response::from_parts(parts, Body::from(bytes))
        }
        Err(err) => {
            // The body is already consumed here; forward empty rather than fail.
            if failed {
                warn!(
                    %method, %uri, status = %parts.status, error = %err,
                    "request failed (response body not buffered)"
                );
            } else {
                debug!("failed to buffer response body for logging: {err}");
            }
            Response::from_parts(parts, Body::empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_otp_and_keeps_the_rest() {
        let out = sanitize_body(br#"{"reference":"abc","otp":"123456"}"#);
        assert_eq!(out, r#"{"reference":"abc","otp":"[redacted]"}"#);
    }

    #[test]
    fn redacts_case_insensitively_and_bare_values() {
        let out = sanitize_body(br#"{"Password": 1234, "email":"a@b.c"}"#);
        assert!(out.contains(r#""Password": "[redacted]""#));
        assert!(out.contains("a@b.c"));
    }

    #[test]
    fn keeps_malformed_json_readable() {
        let out = sanitize_body(br#"{"size": "#);
        assert_eq!(out, r#"{"size": "#);
    }

    #[test]
    fn truncates_long_bodies_without_panicking_on_multibyte() {
        let body = format!(r#"{{"details":"{}"}}"#, "ก".repeat(4000));
        let out = sanitize_body(body.as_bytes());
        assert!(out.chars().count() < 4000);
        assert!(out.contains("truncated"));
    }
}
