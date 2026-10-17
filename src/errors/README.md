# errors

`AppError` (`app_error.rs`) is the single error type every layer returns. It renders the error
envelope through `ToDefaultResponse` (`src/traits/default_response.rs`). Each feature adds its own
enum, and `AppError` delegates to it.

## Rules

- One file per feature: `<name>_error.rs`, with enum `<Name>Error`.
- Codes are `<PREFIX>-NNN` (`APP-`, `PRODUCT-`, …), numbered from `001` per prefix. **Never reuse a
  retired code**, so a code in an old log always means the same thing.
- Status, code and message are decided together in **one** `match` inside `to_default_response`.
- Messages are static and safe to show a client. Every 5xx uses the generic
  `"Internal server error"`; the detail goes to the log through `error!` in the `From` impl.
- Infrastructure failures (`sqlx`, Redis, `serde_json`) already convert into `AppError` through
  `From`, so DAOs and utils just use `?`.

## Code registry

| Code | Variant | Status | Note |
|---|---|---|---|
| `APP-001` | `DefaultError` | 500 | Unclassified |
| `APP-002` | `MissingConfig` | 500 | Startup only |
| `APP-003` | `InvalidConfig` | 500 | Startup only |
| `APP-004` | `Database` | 500 | Any `sqlx::Error` |
| `APP-005` | `Redis` | 500 | Redis command or pool |
| `APP-006` | `Json` | 500 | `serde_json` (e.g. a stale cache entry) |
| `APP-007..009` | — | — | Retired (request-parsing rejections, now axum's own) |
| `APP-010` | `RateLimitError` | 429 | Production only |
| `APP-011` | `Container` | 500 | Local startup only |
| `APP-012` | — | — | Retired (seed data) |
| `APP-013` | `Io` | 500 | Local startup only (reading `src/dao/schemas`) |

Add a row for every code you introduce.

## Template — `src/errors/product_error.rs`

```rust
use std::fmt::{self, Display};

use axum::http::StatusCode;

use crate::{schemas::default::DefaultResponse, traits::default_response::ToDefaultResponse};

use super::app_error::AppError;

#[derive(Debug)]
pub enum ProductError {
    NotFound,
}

impl From<ProductError> for AppError {
    fn from(inner: ProductError) -> Self {
        AppError::Product(inner)
    }
}

impl Display for ProductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProductError::NotFound => write!(f, "not found"),
        }
    }
}

impl ToDefaultResponse<()> for ProductError {
    fn to_default_response(&self) -> (StatusCode, DefaultResponse<()>) {
        let (status, code, message) = match self {
            ProductError::NotFound => (StatusCode::NOT_FOUND, "PRODUCT-001", "Product not found"),
        };
        (status, DefaultResponse::error(code, message))
    }
}
```

## Register

1. `src/errors.rs`: add `pub mod product_error;`.
2. `src/errors/app_error.rs`:
   - `use crate::errors::product_error::ProductError;`
   - add the variant `Product(ProductError),` to `AppError`
   - in `to_default_response`, add the arm
     `AppError::Product(e) => return e.to_default_response(),`. The `return` skips the APP-envelope
     line, so a `PRODUCT-001` never surfaces as an `APP-` code.
   - in `Display`, add `AppError::Product(e) => write!(f, "Product error: {e}"),`
3. Add the new codes to the registry table above.
