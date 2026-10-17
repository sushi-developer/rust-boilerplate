# routes

The HTTP layer. A handler extracts input, calls **one** service function, and wraps the result in
the response envelope. No SQL and no business rules here.

## Rules

- One file per feature: `<name>_route.rs`, exposing `pub fn <name>_route() -> Router<AppState>`.
- Handlers return `Result<Json<DefaultResponse<T>>, AppError>`. Build the body with
  `default_response(t)` (message `"Success"`) or `message_response(t, "…")`.
- Argument order: `State(state): State<AppState>`, then `Path` / `Query`, then the body (`Json`) **last**.
- Use axum's own `Json`, `Path` and `Query`. Malformed input is rejected by axum as plain text
  (400 / 413 / 415 / 422) before the handler runs. Domain validation belongs in the service.
- Every handler has `#[debug_handler]` and `#[utoipa::path]`. `path` is relative to the feature's
  `nest` in `src/docs.rs` (`""` for the collection, `"/{id}"` for one item). Error responses use
  `body = ErrorRes` and name their code in the description.
- One `#[derive(OpenApi)]` struct per file, listing its `paths(...)` and `components(schemas(...))`.

`src/routes.rs` also owns `docs_routes()`, which serves the Scalar UI at `/docs` and the spec at
`/api-docs/openapi.json` — only merged when ENVIRONMENT=staging.

## Template — `src/routes/product_route.rs`

```rust
use axum::{
    Json, debug_handler,
    extract::{Path, State},
    routing::{Router, get},
};
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    AppState,
    errors::app_error::AppError,
    schemas::{
        default::{DefaultResponse, ErrorRes, default_response},
        product_schema::ProductRes,
    },
    services::product_service,
};

pub fn product_route() -> Router<AppState> {
    Router::new().route("/products/{product_id}", get(get_product))
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/{product_id}",
    params(("product_id" = Uuid, Path, description = "Product id")),
    responses(
        (status = 200, description = "The product", body = DefaultResponse<ProductRes>),
        (status = 404, description = "No product with this id (PRODUCT-001)", body = ErrorRes),
        (status = 500, description = "Database failure (APP-004)", body = ErrorRes),
    ),
    tag = "product"
)]
pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DefaultResponse<ProductRes>>, AppError> {
    Ok(default_response(product_service::get_product(&state, &id).await?))
}

#[derive(OpenApi)]
#[openapi(paths(get_product), components(schemas(ProductRes, ErrorRes)))]
pub struct ProductDoc;
```

## Register

1. `src/routes.rs`: add `pub mod product_route;`.
2. `src/routes.rs`, in `build_router`, chain onto the first router so the rate limit covers it:
   `Router::new().route("/", …).merge(routes::product_route::product_route())`.
3. `src/docs.rs`: `use crate::routes::product_route::ProductDoc;` and add
   `nest((path = "/products", api = ProductDoc))` to `#[openapi(...)]`.
