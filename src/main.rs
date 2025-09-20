#![warn(unused_extern_crates)]

use axum::Router;
use config::AppState;
use middleware::ValidateSessionLayer;
use routes::*;
use sqlx::migrate;
use std::{sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
};
use tracing::{event, span, Level};

mod auth;
mod config;
mod default_route_handlers;
mod middleware;
mod routes;
mod utilities;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    // Start tracing
    tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(true)
        .init();
    let span = span!(Level::INFO, "main_span");
    let _ = span.enter();

    let app_state = get_app_state().await;

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
    let assets = ServeDir::new("assets").not_found_service(ServeFile::new("assets/404.html"));
    let protected_routes = get_protected_routes();
    let open_routes = get_open_routes();

    Router::new()
        .merge(protected_routes)
        .layer(ServiceBuilder::new().layer(ValidateSessionLayer::new(state.clone())))
        .merge(open_routes)
        .with_state(state.clone())
        .nest_service("/assets", assets)
        .layer(
            ServiceBuilder::new().layer(TimeoutLayer::new(Duration::from_secs(
                state.config.server.request_timeout,
            ))),
        )
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
