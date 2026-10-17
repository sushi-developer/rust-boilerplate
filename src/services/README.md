# services

The business layer. Validates input, applies rules, turns "not there" into a domain error, and
converts models into response DTOs. It never touches axum types (`Json`, `StatusCode`, extractors).

## Rules

- One file per feature: `<name>_service.rs`.
- Signature: `pub async fn <verb>_<noun>(state: &AppState, …) -> Result<T, AppError>`. Read
  `state.db`, `state.redis` and `state.config`; nothing global.
- Return DTOs (`*Res` from `schemas/`), never models.
- Validate **before** touching the database. Return a domain error such as
  `Err(ProductError::InvalidName)?`. The `?` converts it into `AppError` through `From`.
- Map `Option::None` from a DAO with `.ok_or(ProductError::NotFound)?`.
- Run independent queries concurrently: `tokio::try_join!(dao_a(&state.db, …), dao_b(&state.db, …))?`.
- Multi-statement writes go in one transaction:
  ```rust
  let mut tx = state.db.begin().await?;
  product_dao::insert(&mut *tx, …).await?;
  stock_dao::insert(&mut *tx, …).await?;
  tx.commit().await?;
  ```
- To cache, take a connection with `let mut conn = state.redis.get().await?;` and call
  `utils::cache` (see `src/utils/README.md`).

## Template — `src/services/product_service.rs`

```rust
use uuid::Uuid;

use crate::{
    AppState,
    dao::product_dao,
    errors::{app_error::AppError, product_error::ProductError},
    schemas::product_schema::ProductRes,
};

pub async fn get_product(state: &AppState, id: &Uuid) -> Result<ProductRes, AppError> {
    let product = product_dao::find_by_id(&state.db, id)
        .await?
        .ok_or(ProductError::NotFound)?;
    Ok(ProductRes::from(product))
}
```

## Register

`src/services.rs`: add `pub mod product_service;`.
