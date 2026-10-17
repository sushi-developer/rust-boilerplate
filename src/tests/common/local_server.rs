use std::time::Duration;

use deadpool_redis::Pool as RedisPool;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::utils::mocked_db_connection::{LOCAL_INFRA_FILE, LocalInfra};

const NOT_RUNNING: &str = "local server is not running. Start it first with `cargo run` (ENVIRONMENT=local), then re-run `cargo test`";

pub struct LocalServer {
    pub base_url: String,
    pub http: reqwest::Client,
    pub db: PgPool,
    pub redis: RedisPool,
}

pub async fn connect_local_server() -> LocalServer {
    let raw = std::fs::read_to_string(LOCAL_INFRA_FILE)
        .unwrap_or_else(|_| panic!("{NOT_RUNNING} ({LOCAL_INFRA_FILE} not found)"));
    let infra: LocalInfra = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{LOCAL_INFRA_FILE} is malformed: {e}"));

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let alive = http
        .get(format!("{}/", infra.base_url))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success());
    assert!(
        alive,
        "{NOT_RUNNING} (no response from {}; stale {LOCAL_INFRA_FILE}?)",
        infra.base_url
    );

    let db = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&infra.database_url)
        .await
        .unwrap_or_else(|e| panic!("cannot reach the local server's Postgres: {e}"));
    let redis = deadpool_redis::Config::from_url(&infra.redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();

    LocalServer {
        base_url: infra.base_url,
        http,
        db,
        redis,
    }
}
