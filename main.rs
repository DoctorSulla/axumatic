use std::str::FromStr;

use config::Config;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

mod config;
mod create_tables;
mod middleware;
mod route_handlers;
mod routes;

#[tokio::main]
async fn main() {
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

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
