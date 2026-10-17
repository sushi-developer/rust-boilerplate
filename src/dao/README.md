# dao

SQL only, with one file per table. No validation and no domain errors: a `sqlx::Error` becomes
`AppError::Database` (`APP-004`) through `?`.

## Rules

- One file per table: `<name>_dao.rs`.
- Every function takes `db: impl PgExecutor<'_>`, so the caller can pass `&state.db` (pool) or
  `&mut *tx` (transaction).
- Return types: a model for inserts and updates with `RETURNING`, `Option<Model>` for a lookup by
  key, `Vec<Model>` for lists, and `bool` (`rows_affected() > 0`) for deletes.
- Use runtime queries (`sqlx::query_as::<_, Model>(…)`, `sqlx::query_scalar(…)`), **not** the
  `query!` macros. The build then needs no `DATABASE_URL` and no `.sqlx` offline cache.
- Name columns explicitly through a `COLUMNS` const. Never use `SELECT *`.
- Each table's DDL lives in `src/dao/schemas/<NNN>_<table>.sql`, using only idempotent statements
  (`CREATE TABLE IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`, `ALTER TABLE … ADD COLUMN IF NOT EXISTS`).
  A schema change is a new idempotent statement appended to that file.
- `src/dao.rs::ensure_schema` reads every `*.sql` in `src/dao/schemas` at `local` boot and runs them
  with `sqlx::raw_sql` in file-name order, so a referenced table needs a lower `NNN`. Staging and
  production schemas are applied outside the app. Dao files never define `SCHEMA` or
  `ensure_schema`.

## Template — `src/dao/schemas/001_products.sql`

```sql
CREATE TABLE IF NOT EXISTS products (
    id         UUID PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_products_created_at ON products (created_at DESC, id DESC);
```

## Template — `src/dao/product_dao.rs`

```rust
use sqlx::postgres::PgExecutor;
use uuid::Uuid;

use crate::{errors::app_error::AppError, models::product_model::ProductModel};

const COLUMNS: &str = "id, name, created_at, updated_at";

pub async fn find_by_id(
    db: impl PgExecutor<'_>,
    id: &Uuid,
) -> Result<Option<ProductModel>, AppError> {
    Ok(
        sqlx::query_as::<_, ProductModel>(&format!("SELECT {COLUMNS} FROM products WHERE id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}
```

## Register

1. `src/dao.rs`: add `pub mod product_dao;`.
2. Add `src/dao/schemas/<NNN>_products.sql`, numbered after any table it references. It is
   picked up on the next `local` boot. No code change and no recompile needed.
