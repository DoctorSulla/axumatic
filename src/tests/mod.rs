use crate::config::get_config;
use crate::default_route_handlers::{
    ApiResponse, ChangePassword, LoginDetails, PasswordResetCompleteRequest,
    PasswordResetInitiateRequest, ResponseType,
};
use crate::utilities::generate_unique_id;
use crate::{default_route_handlers::RegistrationDetails, get_app, get_app_state};
use http::header::{CONTENT_TYPE, COOKIE};
use http::{HeaderValue, StatusCode};
use reqwest::header::HeaderMap;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::prelude::FromRow;

#[derive(Serialize, Deserialize, FromRow, Debug)]
struct Code {
    code_type: String,
    email: String,
    code: String,
    created_ts: i64,
    expiry_ts: i64,
    used: bool,
}

async fn run_test_app() -> u16 {
    let state = get_app_state().await;
    let app = get_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    port
}

async fn _cleanup() -> Result<(), anyhow::Error> {
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
    sqlx::query("DELETE FROM USERS WHERE email=$1")
        .bind(&email)
        .execute(&state.db_connection_pool)
        .await?;
    sqlx::query("DELETE FROM CODES WHERE email=$1")
        .bind(&email)
        .execute(&state.db_connection_pool)
        .await?;
    sqlx::query("DELETE FROM SESSIONS WHERE username=$1")
        .bind(&username)
        .execute(&state.db_connection_pool)
        .await?;

    state.db_connection_pool.close().await;
    Ok(())
}

async fn create_valid_reg(port: u16) -> (String, String, String, Response) {
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
        sub: None,
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

async fn login(email: String, password: String, port: u16) -> Option<String> {
    let url = format!("{}:{}/account/login", SERVER_URL, port);
    let client = Client::new();
    let login_details = LoginDetails { email, password };
    let login_json = serde_json::to_string(&login_details).unwrap();
    let response = client
        .post(url)
        .body(login_json)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();

    if let Some(raw_cookie) = response.headers().get("set-cookie") {
        let parts: Vec<&str> = raw_cookie.to_str().unwrap().split(';').collect();
        let (key, value) = parts[0].trim().split_once('=').unwrap();
        if key == "session-key" {
            return Some(value.to_string());
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
async fn login_success() {
    let port = run_test_app().await;
    let (username, email, password, _response) = create_valid_reg(port).await;
    let login = login(email.clone(), password, port).await;
    assert!(login.is_some());
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
        sub: None,
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
        sub: None,
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
        sub: None,
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
        sub: None,
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
        sub: None,
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
        sub: None,
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
    let url = format!("{}:{}/account/changePassword", SERVER_URL, port);
    let (username, email, password, _response) = create_valid_reg(port).await;
    let session_key = login(email.clone(), password.clone(), port).await.unwrap();

    let new_password = generate_unique_id(20);
    let session_cookie = format!("session-key={session_key}");

    let change_password_request = ChangePassword {
        old_password: password.clone(),
        password: new_password.clone(),
        confirm_password: new_password.clone(),
    };

    let body = serde_json::to_string(&change_password_request).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );

    headers.insert(COOKIE, HeaderValue::from_str(&session_cookie).unwrap());

    let response: ApiResponse = client
        .patch(url)
        .body(body)
        .headers(headers)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let test_new_creds = login(email.clone(), new_password.clone(), port).await;

    assert!(test_new_creds.is_some());

    assert_eq!(response.response_type, ResponseType::PasswordChangeSuccess);
    let _ = delete_reg(username, email).await;
}

#[tokio::test]
async fn change_password_and_use_old_creds() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/changePassword", SERVER_URL, port);
    let (username, email, password, _response) = create_valid_reg(port).await;
    let session_key = login(email.clone(), password.clone(), port).await.unwrap();

    let new_password = generate_unique_id(20);
    let session_cookie = format!("session-key={session_key}");

    let change_password_request = ChangePassword {
        old_password: password.clone(),
        password: new_password.clone(),
        confirm_password: new_password.clone(),
    };

    let body = serde_json::to_string(&change_password_request).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );

    headers.insert(COOKIE, HeaderValue::from_str(&session_cookie).unwrap());

    let response: ApiResponse = client
        .patch(url)
        .body(body)
        .headers(headers)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let test_new_creds = login(email.clone(), password.clone(), port).await;

    assert!(test_new_creds.is_none());

    assert_eq!(response.response_type, ResponseType::PasswordChangeSuccess);
    let _ = delete_reg(username, email).await;
}

#[tokio::test]
async fn change_password_invalid_password() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/changePassword", SERVER_URL, port);
    let (username, email, password, _response) = create_valid_reg(port).await;
    let session_key = login(email.clone(), password.clone(), port).await.unwrap();

    let new_password = generate_unique_id(110);
    let session_cookie = format!("session-key={session_key}");

    let change_password_request = ChangePassword {
        old_password: password.clone(),
        password: new_password.clone(),
        confirm_password: new_password.clone(),
    };

    let body = serde_json::to_string(&change_password_request).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str("application/json").unwrap(),
    );

    headers.insert(COOKIE, HeaderValue::from_str(&session_cookie).unwrap());

    let response = client
        .patch(url)
        .body(body)
        .headers(headers)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let _ = delete_reg(username, email).await;
}

#[tokio::test]
async fn reset_password() {
    let port = run_test_app().await;
    let client = Client::new();
    let url = format!("{}:{}/account/resetPassword", SERVER_URL, port);
    let (username, email, _password, _response) = create_valid_reg(port).await;

    let initiate_reset_password_request = PasswordResetInitiateRequest(email.clone());
    let body = serde_json::to_string(&initiate_reset_password_request).unwrap();

    let _ = client
        .post(&url)
        .body(body)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap();

    let pool = get_config().get_db_pool().await;

    let code = sqlx::query_as::<_, Code>("SELECT code,email,code_type,created_ts,expiry_ts,used FROM codes WHERE email=$1 AND code_type='PasswordReset' AND used=false")
        .bind(&email)
        .fetch_optional(&pool)
        .await.unwrap().unwrap();

    let new_password = generate_unique_id(25);

    let complete_reset_password_request = PasswordResetCompleteRequest {
        code: code.code,
        password: new_password.clone(),
        confirm_password: new_password.clone(),
    };

    let body = serde_json::to_string(&complete_reset_password_request).unwrap();

    let complete_reset_response: ApiResponse = client
        .patch(&url)
        .body(body)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(
        complete_reset_response.response_type,
        ResponseType::PasswordResetSuccess
    );

    let _ = delete_reg(username, email).await;
}

#[tokio::test]
async fn check_max_login_attempts() {
    let port = run_test_app().await;
    let client = Client::new();
    let config = get_config();

    let (username, email, password, _response) = create_valid_reg(port).await;
    let mut i: i32 = 0;

    while i < config.server.max_unsuccessful_login_attempts {
        let _ = login(email.clone(), "incorrect_password".to_string(), port).await;
        i += 1;
    }

    let url = format!("{}:{}/account/login", SERVER_URL, port);
    let login_details = LoginDetails {
        email: email.clone(),
        password,
    };
    let login_json = serde_json::to_string(&login_details).unwrap();
    let response: ApiResponse = client
        .post(url)
        .body(login_json)
        .header("Content-Type", "application/json")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response.response_type, ResponseType::Error);
    assert_eq!(
        response.message,
        *"Too many login attempts, please reset your password"
    );

    let _ = delete_reg(username, email).await;
}
