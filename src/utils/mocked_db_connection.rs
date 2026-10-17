use deadpool_redis::Pool as RedisPool;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use testcontainers_modules::{
    postgres::Postgres,
    redis::Redis,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};
use tracing::{error, info};

use crate::config::Config;
use crate::dao;
use crate::errors::app_error::AppError;
use crate::utils::db_connection::{get_database, get_redis_pool};

pub const LOCAL_INFRA_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/local-infra.json");

#[derive(Serialize, Deserialize)]
pub struct LocalInfra {
    pub base_url: String,
    pub database_url: String,
    pub redis_url: String,
}

pub struct MockContainers {
    postgres: ContainerAsync<Postgres>,
    redis: ContainerAsync<Redis>,
    database_url: String,
    redis_url: String,
}

impl MockContainers {
    pub fn write_infra_file(&self, port: u16) -> std::io::Result<()> {
        let infra = LocalInfra {
            base_url: format!("http://127.0.0.1:{port}"),
            database_url: self.database_url.clone(),
            redis_url: self.redis_url.clone(),
        };
        std::fs::write(LOCAL_INFRA_FILE, serde_json::to_string_pretty(&infra)?)
    }

    pub async fn rm(self) {
        let _ = std::fs::remove_file(LOCAL_INFRA_FILE);
        info!("Removing local mock Postgres container...");
        if let Err(e) = self.postgres.rm().await {
            error!("Failed to remove mock Postgres container: {e}");
        }
        info!("Removing local mock Redis container...");
        if let Err(e) = self.redis.rm().await {
            error!("Failed to remove mock Redis container: {e}");
        }
    }
}

pub async fn get_mocked_infra(
    cfg: &Config,
) -> Result<(MockContainers, PgPool, RedisPool), AppError> {
    let mut local_cfg = cfg.clone();

    let postgres = Postgres::default()
        .with_user("postgres")
        .with_password("postgres")
        .with_db_name("app")
        .start()
        .await?;
    let pg_port = postgres.get_host_port_ipv4(5432).await?;
    local_cfg.database_url = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/app");

    let redis = Redis::default().start().await?;
    let redis_port = redis.get_host_port_ipv4(6379).await?;
    local_cfg.redis_url = format!("redis://127.0.0.1:{redis_port}");

    let db = get_database(&local_cfg).await?;
    dao::ensure_schema(&db).await?;
    let redis_pool = get_redis_pool(&local_cfg).await?;

    info!("Created mocked postgres + redis");
    Ok((
        MockContainers {
            postgres,
            redis,
            database_url: local_cfg.database_url,
            redis_url: local_cfg.redis_url,
        },
        db,
        redis_pool,
    ))
}
