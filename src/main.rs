use std::{net::SocketAddr, sync::Arc};

use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::info;
use utoipa::OpenApi;

use crate::{
    config::{Config, Environment},
    docs::ApiDoc,
    routes::build_router,
    utils::{db_connection, mocked_db_connection},
};

mod config;
mod dao;
mod docs;
mod errors;
mod logger;
mod middlewares;
mod models;
mod routes;
mod schemas;
mod services;
mod traits;
mod utils;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct AppState {
    // PgPool and RedisPool are Arc inside — no extra Arc
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() {
    // `cargo openapi > openapi.json` — print the spec and exit before touching any infra.
    if std::env::args().nth(1).as_deref() == Some("openapi") {
        let spec = ApiDoc::openapi()
            .to_pretty_json()
            .expect("serialize OpenAPI document");
        println!("{spec}");
        return;
    }

    dotenvy::dotenv().ok();
    logger::setup_tracing();

    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let (mock, db, redis) = if config.environment == Environment::Local {
        let (mock, db, redis) = mocked_db_connection::get_mocked_infra(&config)
            .await
            .expect("Failed to start local Postgres + Redis containers");
        (Some(mock), db, redis)
    } else {
        let db = db_connection::get_database(&config)
            .await
            .expect("Failed to connect to Postgres");
        let redis = db_connection::get_redis_pool(&config)
            .await
            .expect("Failed to connect to Redis");
        (None, db, redis)
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let state_port = config.port;
    let state = AppState {
        db,
        redis,
        config: Arc::new(config),
    };
    let app = build_router(state);

    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    if let Some(mock) = &mock {
        mock.write_infra_file(state_port)
            .expect("Failed to write target/local-infra.json");
    }
    info!("Listening on http://{addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("Server failed");

    info!("Server is shutting down. Cleaning up resources...");
    if let Some(mock) = mock {
        mock.rm().await;
    }
    info!("Graceful shutdown complete!");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to register signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
