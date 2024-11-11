use http::StatusCode;
use reqwest::Client;
use serde::Serialize;
use serde_json;
use sqlx::migrate;

use crate::{get_app_state, route_handlers::RegistrationDetails, AppState};

const SERVER_URL: &str = "http://localhost:3000";

#[tokio::test]

async fn migrations() {
    let app_state = get_app_state().await;
    let migrations = migrate!().run(&app_state.db_connection_pool).await;
    assert!(migrations.is_ok());
}

async fn register() {
    let client = reqwest::Client::new();
    let url = format!("{}/account/register", SERVER_URL);
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
}
