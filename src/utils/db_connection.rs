use std::time::Duration;

use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::config::Config;
use crate::errors::app_error::AppError;

pub async fn get_database(cfg: &Config) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(cfg.database_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await?;
    tracing::info!("Connected to postgres database");
    Ok(pool)
}

pub async fn get_redis_pool(cfg: &Config) -> Result<RedisPool, AppError> {
    let pool = deadpool_redis::Config::from_url(&cfg.redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
    let mut conn = pool.get().await?;
    deadpool_redis::redis::cmd("PING")
        .query_async::<()>(&mut conn)
        .await?;
    tracing::info!("Connected to redis");
    Ok(pool)
}
