# middlewares

Code that wraps requests: logging today, and auth or request ids when a project needs them.
Middlewares are attached in `build_router` (`src/routes.rs`).

## What runs today

`logging.rs`, `log_request_response`:
- `local`: `debug!` of every request and response body.
- Every environment: `warn!` of the request and response body for each 4xx/5xx, including axum's
  own extractor rejections.
- Bodies are redacted (`otp`, `password`, `token`, `access_token`, `refresh_token`, `secret`,
  `authorization`) and truncated to 2048 chars. Only text bodies up to 64 KiB with a declared
  `Content-Length` are buffered; multipart, binary and chunked bodies pass through unlogged.

## Layer order

In axum, each `.layer(…)` wraps everything added before it, so the **last** layer runs **first**.
For a request, the current order is:

```
TraceLayer
 └─ referrer-policy: no-referrer
     └─ x-frame-options: DENY
         └─ x-content-type-options: nosniff
             └─ log_request_response
                 └─ GovernorLayer  (production only; feature routes, not /docs)
                     └─ handler
```

## Rules

- One file per middleware: `<name>.rs`.
- Signature: `pub async fn <name>(State(state): State<AppState>, req: Request, next: Next) -> Response`.
  Drop the `State` argument if it isn't needed. To reject a request with the error envelope, return
  `Result<Response, AppError>` instead.
- Read configuration from `state.config`, never with `std::env::var`.
- Whole app: `.layer(middleware::from_fn_with_state(state.clone(), …))` in `build_router`.
  Choose its position from the diagram above.
- Some routes only (for example an auth guard): `.route_layer(middleware::from_fn(…))` on that
  route's method router inside the feature's `*_route.rs`.

## Template — `src/middlewares/request_id.rs`

```rust
use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub async fn request_id(req: Request, next: Next) -> Response {
    let id = Uuid::now_v7().to_string();
    let mut res = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        res.headers_mut().insert("x-request-id", value);
    }
    res
}
```

## Register

1. `src/middlewares.rs`: add `pub mod request_id;`.
2. `src/routes.rs`, in `build_router`: `.layer(middleware::from_fn(middlewares::request_id::request_id))`
   at the position you want in the chain.
