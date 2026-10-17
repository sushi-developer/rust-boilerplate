# Rust API boilerplate

```
route  (src/routes)    HTTP: axum extractors, OpenAPI annotations, response envelope
  └─ service (src/services)  validation, business rules, domain errors
       └─ dao (src/dao)       SQL only, `impl PgExecutor<'_>` (pool or transaction)
```

## Run

```sh
cp .env.example .env
cargo run
```

`ENVIRONMENT=local` (the default in `.env.example`) needs Docker: it starts throwaway, empty
Postgres and Redis containers. Both are removed on Ctrl-C.

For `staging` / `production`, set `DATABASE_URL` and `REDIS_URL`. Tables are **not** created on boot; apply `src/dao/schemas/*.sql` before deploying.

## Test

```sh
cargo run     # terminal 1 — ENVIRONMENT=local, keep it running
cargo test    # terminal 2 — tests hit that server's HTTP, Postgres and Redis
```

Without a running local server, integration tests fail with a 'local server is not running' panic.
Tests share the dev database, so use fresh ids (`Uuid::now_v7()`) and never assume an empty table.

## API docs

Served only when ENVIRONMENT=staging; elsewhere use the offline dump.

- Scalar UI: `<staging-host>/docs`
- JSON: `<staging-host>/api-docs/openapi.json`
- Offline dump: `cargo openapi > openapi.json`

## Add a feature

Each layer folder has a `README.md` with its rules, a template and where to register a new file.
Work bottom-up:

1. [`src/models`](src/models/README.md): the stored row
2. [`src/dao`](src/dao/README.md): SQL and the table schema
3. [`src/schemas`](src/schemas/README.md): request/response DTOs and the envelope
4. [`src/errors`](src/errors/README.md): domain errors and the code registry
5. [`src/services`](src/services/README.md): validation and business rules
6. [`src/routes`](src/routes/README.md): handlers, OpenAPI, and registration

Cross-cutting: [`src/middlewares`](src/middlewares/README.md) and [`src/utils`](src/utils/README.md).
