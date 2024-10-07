use std::str::FromStr;

use config::Config;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tracing::{event, span, Level};
use tracing_subscriber;

mod config;
mod create_tables;
mod middleware;
mod route_handlers;
mod routes;
mod tests;

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::FmtSubscriber::builder()
        .with_ansi(true)
        .init();
    let span = span!(Level::INFO, "main_span");
    let _ = span.enter();

    event!(Level::INFO, "Creating database");
    let config = Config {
        database_file: "./database.db",
    };

    let app = routes::get_routes();

    let connection_options = SqliteConnectOptions::from_str(config.database_file)
        .expect("Unable to parse connection url")
        .create_if_missing(true);

    let connection_pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await;

    if let Err(e) = connection_pool {
        panic!("{}", e);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
