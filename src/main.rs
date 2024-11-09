use axum::Router;
use config::Config;
use create_tables::create_tables;
use lettre::SmtpTransport;
use middleware::ValidateSessionLayer;
use routes::{get_open_routes, get_protected_routes};
use sqlx::{Pool, Sqlite};
use std::{sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
};
use tracing::{event, span, Level};

mod config;
mod create_tables;
mod middleware;
mod route_handlers;
mod routes;
mod tests;
mod utilities;

#[derive(Clone)]
struct AppState {
    db_connection_pool: Pool<Sqlite>,
    email_connection_pool: SmtpTransport,
    config: Config,
}

#[tokio::main]
async fn main() {
    // Start tracing
    tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(true)
        .init();
    let span = span!(Level::INFO, "main_span");
    let _ = span.enter();

    event!(Level::INFO, "Getting config from file");
    let config = config::get_config();

    event!(Level::INFO, "Creating email connection pool");
    let email_connection_pool = config.get_email_pool();

    event!(Level::INFO, "Creating database connection pool");
    let db_connection_pool = config.get_db_pool().await;

    let app_state = Arc::new(AppState {
        db_connection_pool,
        email_connection_pool,
        config,
    });

    create_tables(app_state.db_connection_pool.clone())
        .await
        .expect("Unable to create tables");

    let assets = ServeDir::new("assets").not_found_service(ServeFile::new("assets/404.html"));
    let protected_routes = get_protected_routes();
    let open_routes = get_open_routes();

    let app = Router::new()
        .merge(protected_routes)
        .layer(ServiceBuilder::new().layer(ValidateSessionLayer::new(app_state.clone())))
        .merge(open_routes)
        .with_state(app_state.clone())
        .nest_service("/assets", assets)
        .layer(
            ServiceBuilder::new().layer(TimeoutLayer::new(Duration::from_secs(
                app_state.config.server.request_timeout,
            ))),
        );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", app_state.config.server.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
