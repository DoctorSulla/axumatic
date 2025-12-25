#![warn(unused_extern_crates)]

use axum::Router;
use config::AppState;
use http::StatusCode;
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
use tracing::{Level, event, span};

use crate::utilities::start_session_cleaner;
mod auth;
mod config;
mod default_route_handlers;
mod middleware;
mod routes;
mod user;
mod utilities;

#[cfg(test)]
mod tests;

static NONCE_STORE: LazyLock<Arc<RwLock<HashMap<String, i64>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

#[tokio::main]
async fn main() {
    // Start tracing
    tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(true)
        .init();
    let span = span!(Level::INFO, "main_span");
    let _ = span.enter();

    let app_state = get_app_state().await;

    start_session_cleaner(app_state.clone()).await;

    event!(Level::INFO, "Creating tables");

    migrations(app_state.clone())
        .await
        .expect("Couldn't complete migrations");

    let app = get_app(app_state.clone());

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", app_state.config.server.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub fn get_app(state: Arc<AppState>) -> Router {
    let protected_routes = get_protected_routes();
    let open_routes = get_open_routes();

    Router::new()
        .merge(protected_routes)
        .layer(ServiceBuilder::new().layer(ValidateSessionLayer::new(state.clone())))
        .merge(open_routes)
        .with_state(state.clone())
        .layer(ServiceBuilder::new().layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(state.config.server.request_timeout),
        )))
        .layer(ServiceBuilder::new().layer(CorsLayer::very_permissive()))
}

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

pub async fn migrations(state: Arc<AppState>) -> Result<(), anyhow::Error> {
    migrate!().run(&state.db_connection_pool).await?;
    Ok(())
}
