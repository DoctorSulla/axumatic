use config::Config;
use create_tables::create_tables;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{event, span, Level};
use tracing_subscriber;

mod config;
mod create_tables;
mod middleware;
mod route_handlers;
mod routes;
mod tests;
mod utilities;

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

    let assets = ServeDir::new("assets").not_found_service(ServeFile::new("assets/404.html"));

    let app = routes::get_routes().nest_service("/assets", assets);

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

    create_tables(connection_pool)
        .await
        .expect("Unable to create tables");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
