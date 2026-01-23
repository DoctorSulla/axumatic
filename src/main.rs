#![warn(unused_extern_crates)]

use axumatic::{get_app, get_app_state, migrations, utilities::start_session_cleaner};
use tracing::{Level, event, span};

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
