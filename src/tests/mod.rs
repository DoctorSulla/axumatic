use http::StatusCode;
use reqwest::Client;
use serde::Serialize;
use serde_json;
use sqlx::migrate;

use crate::{get_app, get_app_state, migrations, route_handlers::RegistrationDetails, AppState};

async fn run_test_app() -> u16 {
    let state = get_app_state().await;
    migrations(state.clone())
        .await
        .expect("Unable to complete migrations");
    let app = get_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    port
}

async fn cleanup() -> Result<(), anyhow::Error> {
    let state = get_app_state().await;
    sqlx::query("DELETE FROM USERS")
        .execute(&state.db_connection_pool)
        .await?;
    sqlx::query("DELETE FROM CODES")
        .execute(&state.db_connection_pool)
        .await?;
    sqlx::query("DELETE FROM SESSIONS")
        .execute(&state.db_connection_pool)
        .await?;
    Ok(())
}

const SERVER_URL: &str = "http://localhost";

#[tokio::test]
async fn register() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "JohnDoe".to_string(),
        email: "john@doe.gmail.com".to_string(),
        password: "TestPassword".to_string(),
        confirm_password: "TestPassword".to_string(),
    };
    let registration_request = serde_json::to_string(&registration_request).unwrap();

    let response = client
        .post(url)
        .body(registration_request)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let _ = cleanup().await;
}

#[tokio::test]
async fn register_invalid_username() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "Joh".to_string(),
        email: "john@doe.gmail.com".to_string(),
        password: "TestPassword".to_string(),
        confirm_password: "TestPassword".to_string(),
    };
    let registration_request = serde_json::to_string(&registration_request).unwrap();

    let response = client
        .post(url)
        .body(registration_request)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let _ = cleanup().await;
}
