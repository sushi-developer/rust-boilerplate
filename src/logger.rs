use std::env;

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn setup_tracing() {
    let trace_level = env::var("TRACE_LEVEL").unwrap_or_else(|_| "info".to_string());
    let filter = EnvFilter::from(trace_level);

    tracing_subscriber::Registry::default()
        .with(filter)
        .with(fmt::layer())
        .init();
}
