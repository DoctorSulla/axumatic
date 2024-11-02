use config::Config;
use create_tables::create_tables;
use routes::get_routes;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::fs::File;
use std::io::prelude::*;
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

    // Open and parse the config file
    let mut file = File::open("./config.toml").expect("Couldn't open config file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Couldn't convert config file to string");

    let config: Config = toml::from_str(contents.as_str()).expect("Couldn't parse config");

    // Create the database and connection pool
    event!(Level::INFO, "Creating database");
    let connection_options = SqliteConnectOptions::from_str(&config.database.file)
        .expect("Unable to open or create database")
        .create_if_missing(true);

    let connection_pool = match SqlitePoolOptions::new()
        .max_connections(config.database.pool_size)
        .connect_with(connection_options)
        .await
    {
        Ok(val) => val,
        Err(e) => panic!("Unable to create connection pool due to {}", e),
    };

    let app_state = Arc::new(AppState {
        connection_pool,
        config,
    });

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
