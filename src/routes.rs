use std::time::Duration;

use axum::{
    Json, Router,
    http::{HeaderValue, header},
    middleware,
    response::IntoResponse,
    routing::get,
};
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::{set_header::SetResponseHeaderLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::{
    AppState, config::Environment, docs::ApiDoc, errors::app_error::AppError, middlewares,
};

pub fn docs_routes() -> Router<AppState> {
    Router::new()
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .route(
            "/api-docs/openapi.json",
            get(|| async { Json(ApiDoc::openapi()) }),
        )
}

pub fn build_router(state: AppState) -> Router {
    let mut app = Router::new().route("/", get(|| async { "OK" }));

    if state.config.environment != Environment::Local {
        let governor_conf = GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(state.config.rate_limit_per_second)
            .burst_size(state.config.rate_limit_burst)
            .finish()
            .expect("Invalid rate limit configuration");

        let limiter = governor_conf.limiter().clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            loop {
                tick.tick().await;
                limiter.retain_recent();
            }
        });

        app = app.layer(
            GovernorLayer::new(governor_conf).error_handler(|e| AppError::from(e).into_response()),
        );
    }

    // Docs are staging-only and aren't rate limited;
    if state.config.environment != Environment::Production {
        app = app.merge(docs_routes());
    }

    app.layer(middleware::from_fn_with_state(
        state.clone(),
        middlewares::logging::log_request_response,
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    ))
    .layer(TraceLayer::new_for_http())
    .with_state(state)
}
