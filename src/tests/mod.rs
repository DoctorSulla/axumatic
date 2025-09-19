use crate::default_route_handlers::LoginDetails;
use crate::utilities::generate_unique_id;
use crate::{
    default_route_handlers::{AuthAndLoginResponse, RegistrationDetails, ResponseType},
    get_app, get_app_state, migrations,
};
use cookie::Cookie;
use http::StatusCode;
use reqwest::{Client, Response};
use serde_json;

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

async fn delete_reg(username: String, email: String) -> Result<(), anyhow::Error> {
    let state = get_app_state().await;
    sqlx::query("DELETE FROM USERS WHERE email=?")
        .bind(&email)
        .execute(&state.db_connection_pool)
        .await?;
    sqlx::query("DELETE FROM CODES WHERE email=?")
        .bind(&email)
        .execute(&state.db_connection_pool)
        .await?;
    sqlx::query("DELETE FROM SESSIONS WHERE username=?")
        .bind(&username)
        .execute(&state.db_connection_pool)
        .await?;
    Ok(())
}

async fn create_valid_reg(port: u16) -> (String, String, String, Response) {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let username = generate_unique_id(20);
    let email = format!("{}@{}.com", username, generate_unique_id(10));
    let password = generate_unique_id(30);
    let registration_request = RegistrationDetails {
        username: username.clone(),
        email: email.clone(),
        password: password.clone(),
        confirm_password: password.clone(),
    };
    let registration_request = serde_json::to_string(&registration_request).unwrap();

    let response = client
        .post(url)
        .body(registration_request)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();
    (username, email, password, response)
}

async fn login(email: String, password: String) -> Option<String> {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/login", SERVER_URL, port);
    let login_details = LoginDetails { email, password };
    let login_json = serde_json::to_string(&login_details).unwrap();
    let response = client
        .post(url)
        .body(login_json)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();

    let response_headers = response.headers();

    if let Some(cookies) = response_headers.get("cookie") {
        for cookie_string in cookies.to_str().unwrap().split(';') {
            let cookie = Cookie::parse(cookie_string).unwrap();
            if cookie.name() == "session-key" {
                return Some(cookie.value().to_string());
            }
        }
    }

    None
}

const SERVER_URL: &str = "http://localhost";

#[tokio::test]
async fn register() {
    let port = run_test_app().await;
    let (username, email, _password, response) = create_valid_reg(port).await;
    assert_eq!(response.status(), StatusCode::OK);
    let _ = delete_reg(username, email).await;
}

#[tokio::test]
async fn register_username_too_short() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "Jo".to_string(),
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
}

#[tokio::test]
async fn register_username_too_long() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "aabcdefghijklmnopqrstuvwxyabcdefghijklmnopqrstuvwxyabcdefghijklmnopqrstuvwxybcdefghijklmnopqrstuvwxya".to_string(),
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
}

#[tokio::test]
async fn register_invalid_email() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "JohnDoe".to_string(),
        email: "johndoe.gmail.com".to_string(),
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
}

#[tokio::test]
async fn register_non_matching_passwords() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "JohnDoe".to_string(),
        email: "john@doe.gmail.com".to_string(),
        password: "TestPasswor".to_string(),
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
}

#[tokio::test]
async fn register_password_too_short() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "JohnDoe".to_string(),
        email: "john@doe.gmail.com".to_string(),
        password: "TestPas".to_string(),
        confirm_password: "TestPas".to_string(),
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
}

#[tokio::test]
async fn register_password_too_long() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "JohnDoe".to_string(),
        email: "john@doe.gmail.com".to_string(),
        password: "aabcdefghijklmnopqrstuvwxyabcdefghijklmnopqrstuvwxyabcdefghijklmnopqrstuvwxybcdefghijklmnopqrstuvwxya".to_string(),
        confirm_password: "aabcdefghijklmnopqrstuvwxyabcdefghijklmnopqrstuvwxyabcdefghijklmnopqrstuvwxybcdefghijklmnopqrstuvwxya".to_string(),
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
}

#[tokio::test]
async fn change_password() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/register", SERVER_URL, port);
    let registration_request = RegistrationDetails {
        username: "JohnDoe".to_string(),
        email: "john@doe.gmail.com".to_string(),
        password: "Qwertyuio123!".to_string(),
        confirm_password: "Qwertyuio123!".to_string(),
    };
    let registration_request = serde_json::to_string(&registration_request).unwrap();

    let response: AuthAndLoginResponse = client
        .post(url)
        .body(registration_request)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response.response_type, ResponseType::RegistrationSuccess);

    let _ = cleanup().await;
}
