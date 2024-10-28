use config::Config;
use create_tables::create_tables;
use routes::get_routes;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use std::{str::FromStr, time::Duration};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::timeout::TimeoutLayer;
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
    connection_pool: Pool<Sqlite>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(true)
        .init();
    let span = span!(Level::INFO, "main_span");
    let _ = span.enter();

    event!(Level::INFO, "Creating database");
    let config = Config {
        database_file: "./database.db",
    };

    let connection_options = SqliteConnectOptions::from_str(config.database_file)
        .expect("Unable to parse connection url")
        .create_if_missing(true);

    let connection_pool = match SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await
    {
        Ok(val) => val,
        Err(e) => panic!("Unable to create connection pool due to {}", e),
    };

    let app_state = Arc::new(AppState { connection_pool });

    create_tables(app_state.connection_pool.clone())
        .await
        .expect("Unable to create tables");

    let assets = ServeDir::new("assets").not_found_service(ServeFile::new("assets/404.html"));
    let app = get_routes()
        .with_state(app_state)
        .nest_service("/assets", assets)
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
