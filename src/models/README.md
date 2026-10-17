# models

The row exactly as it is stored. Models are read and written by `dao/` and converted to DTOs in
`schemas/`. They are never serialized straight into a response.

## Rules

- One file per table: `<name>_model.rs`, with struct `<Name>Model`.
- Derive `Debug, Clone, Serialize, Deserialize, sqlx::FromRow`. Field names equal column names.
- Column → Rust type:

  | Postgres | Rust |
  |---|---|
  | `UUID` | `uuid::Uuid` (generate with `Uuid::now_v7()`) |
  | `TEXT` | `String` |
  | `BIGINT` / `INTEGER` | `i64` / `i32` |
  | `DOUBLE PRECISION` | `f64` |
  | `BOOLEAN` | `bool` |
  | `TIMESTAMPTZ` | `chrono::DateTime<chrono::Utc>` |
  | `JSONB` | `sqlx::types::Json<T>` |
  | any nullable column | `Option<T>` |

- Value types stored inside a `JSONB` column go in `src/models/custom/` (for example
  `custom/price_model.rs`).

## Template — `src/models/product_model.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductModel {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

## Register

`src/models.rs`: add `pub mod product_model;`.
