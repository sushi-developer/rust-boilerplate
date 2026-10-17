# utils

Infrastructure helpers with no HTTP or business knowledge: connections, the local containers,
the cache, and clients for outside services.

## What's here

| File | Provides |
|---|---|
| `db_connection.rs` | `get_database(&Config)`: Postgres pool, 5 s acquire timeout. `get_redis_pool(&Config)`: Redis pool, `PING`ed at startup. |
| `mocked_db_connection.rs` | `get_mocked_infra(&Config)`: `ENVIRONMENT=local` only. Starts throwaway Postgres and Redis containers and returns `MockContainers` with both pools, and runs `dao::ensure_schema`; `write_infra_file` publishes `target/local-infra.json` for the tests, removed by `rm`. `main` must keep `MockContainers` alive and call `.rm().await` on shutdown, because dropping it stops the containers. |
| `cache.rs` | Typed JSON cache over Redis: `get`, `set` (optional TTL in seconds), `del`, `keys` (`SCAN` with a pattern), and `values` (`MGET`). |

## Using the cache

```rust
use crate::utils::cache;

let mut conn = state.redis.get().await?;
if let Some(hit) = cache::get::<ProductRes>(&mut conn, &format!("product:{id}")).await? {
    return Ok(hit);
}
let product = /* load from the DAO */;
cache::set(&mut conn, &format!("product:{id}"), &product, Some(300)).await?;
```

The cached type must derive `Deserialize` as well as `Serialize`. `ProductRes` in the schemas
template derives only `Serialize`, so add `Deserialize` to anything you cache.
A `Redis` failure is `APP-005`, and a cached value that no longer deserializes is `APP-006`.
Delete the key (`cache::del`) when the underlying row changes.

## Rules

- Every function returns `Result<_, AppError>`. Add a `From<…> for AppError` in
  `src/errors/app_error.rs` for each new library error type.
- Take settings as `&Config`. Add new variables to `src/config.rs` (validated at boot) and to
  `.env.example`, never `std::env::var` here.
- A client for an outside service goes in its own `<service>.rs` (e.g. `mail.rs`). If it holds a
  pool or connection, build it once in `main` and add it to `AppState` in `src/main.rs`.

## Register

`src/utils.rs`: add `pub mod <file>;`.
