use axum::Router;
use config::Config;
use create_tables::create_tables;
use lettre::{
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        PoolConfig,
    },
    SmtpTransport,
};
use middleware::ValidateSessionLayer;
use routes::{get_open_routes, get_protected_routes};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::{fs::File, io::prelude::*, str::FromStr, sync::Arc, time::Duration};
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

    // Open and parse the config file
    let mut file = File::open("./config.toml").expect("Couldn't open config file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Couldn't convert config file to string");

    let config: Config = toml::from_str(contents.as_str()).expect("Couldn't parse config");

    // Create TLS transport on port 587 with STARTTLS
    let email_connection_pool = SmtpTransport::starttls_relay(&config.email.server_url)
        .expect("Unable to create email connection pool")
        // Add credentials for authentication
        .credentials(Credentials::new(
            config.email.username.to_owned(),
            config.email.password.to_owned(),
        ))
        // Configure expected authentication mechanism
        .authentication(vec![Mechanism::Plain])
        // Connection pool settings
        .pool_config(PoolConfig::new().max_size(config.email.pool_size))
        .build();

    // Create the database and connection pool
    event!(Level::INFO, "Creating database");
    let connection_options = SqliteConnectOptions::from_str(&config.database.file)
        .expect("Unable to open or create database")
        .create_if_missing(true);

    let db_connection_pool = match SqlitePoolOptions::new()
        .max_connections(config.database.pool_size)
        .connect_with(connection_options)
        .await
    {
        Ok(val) => val,
        Err(e) => panic!("Unable to create connection pool due to {}", e),
    };

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
