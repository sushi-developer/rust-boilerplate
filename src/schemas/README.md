# schemas

The API's request and response types, plus the shared envelope in `default.rs`.

## The envelope (`default.rs`)

Every JSON response has the same shape:

```json
{ "status": "success", "code": "200",      "message": "Success",          "data": { … } }
{ "status": "error",   "code": "APP-004",  "message": "Internal server error", "data": null }
```

| Item | Use |
|---|---|
| `DefaultResponse<T>` | The envelope. `data` is `Option<T>` and is always `null` on error. |
| `default_response(t)` | Success, message `"Success"`. |
| `message_response(t, "…")` | Success with a custom message, e.g. `"Product created"`. |
| `DefaultResponse::error(code, msg)` | Used by error enums in `errors/`; handlers never call it. |
| `TotalPagination<T>` | `{ "result": [...], "total": n }`. The client derives "has more" from `offset + result.len() < total`. |
| `ErrorRes` | Documentation-only mirror of the error body, for `#[utoipa::path]` responses. |
| `default_offset()` / `default_limit()` | `0` / `10`, for `#[serde(default = …)]` on query structs. |

## Rules

- One file per feature: `<name>_schema.rs`.
- Request bodies: `<Verb><Name>Req`, deriving `Deserialize, ToSchema`.
- Query strings: derive `Deserialize, IntoParams` with `#[into_params(parameter_in = Query)]`, and
  document them with `params(<Struct>)` in `#[utoipa::path]`.
- Responses: `<Name>Res`, deriving `Serialize, ToSchema`, plus `impl From<<Name>Model> for <Name>Res`.
  This is the only place a model turns into API output, so internal columns never leak.
- Add `#[schema(example = …)]` / `#[param(example = …)]` so the Scalar UI is useful.

## Template — `src/schemas/product_schema.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::models::product_model::ProductModel;

#[derive(Deserialize, ToSchema)]
pub struct CreateProductReq {
    #[schema(example = "Notebook")]
    pub name: String,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetProductsReq {
    #[param(example = 0)]
    #[serde(default = "crate::schemas::default::default_offset")]
    pub offset: i64,
    #[param(example = 10)]
    #[serde(default = "crate::schemas::default::default_limit")]
    pub limit: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ProductRes {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProductModel> for ProductRes {
    fn from(m: ProductModel) -> Self {
        Self { id: m.id, name: m.name, created_at: m.created_at, updated_at: m.updated_at }
    }
}
```

## Register

`src/schemas.rs`: add `pub mod product_schema;`.
