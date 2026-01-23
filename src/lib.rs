#![warn(unused_extern_crates)]

use axum::Router;
use config::AppState;
use middleware::ValidateSessionLayer;
use routes::*;
use sqlx::migrate;
use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, RwLock},
};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer};
use tracing::{Level, event};

pub mod auth;
pub mod config;
pub mod default_route_handlers;
pub mod middleware;
pub mod routes;
pub mod user;
pub mod utilities;

static NONCE_STORE: LazyLock<Arc<RwLock<HashMap<String, i64>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

pub fn get_app(state: Arc<AppState>) -> Router {
    let protected_routes = get_protected_routes();
    let open_routes = get_open_routes();

    Router::new()
        .merge(protected_routes)
        .layer(ServiceBuilder::new().layer(ValidateSessionLayer::new(state.clone())))
        .merge(open_routes)
        .with_state(state.clone())
        .layer(
            ServiceBuilder::new().layer(TimeoutLayer::new(Duration::from_secs(
                state.config.server.request_timeout,
            ))),
        )
        .layer(ServiceBuilder::new().layer(CorsLayer::very_permissive()))
}

/// Creates the application state with database and email connection pools.
///
/// This function is feature-gated behind `test-utils` for integration testing.
/// The binary accesses this directly.
#[cfg_attr(not(feature = "test-utils"), doc(hidden))]
pub async fn get_app_state() -> Arc<AppState> {
    event!(Level::INFO, "Getting config from file");
    let config = config::get_config();

    event!(Level::INFO, "Creating email connection pool");
    let email_connection_pool = config.get_email_pool();

    event!(Level::INFO, "Creating database connection pool");
    let db_connection_pool = config.get_db_pool().await;

    Arc::new(AppState {
        db_connection_pool,
        email_connection_pool,
        config,
    })
}

/// Runs database migrations.
///
/// This function is feature-gated behind `test-utils` for integration testing.
/// The binary accesses this directly.
#[cfg_attr(not(feature = "test-utils"), doc(hidden))]
pub async fn migrations(state: Arc<AppState>) -> Result<(), anyhow::Error> {
    migrate!().run(&state.db_connection_pool).await?;
    Ok(())
}
