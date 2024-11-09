use axum::extract::{Json, State};
use axum::response::IntoResponse;
use axum::{http::StatusCode, response::Html};
use chrono::Utc;
use cookie::time::Duration;
use cookie::Cookie;
use http::header;
use http::header::HeaderMap;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use thiserror::Error;
use tracing::{event, Level};
use validations::*;

use crate::utilities::*;
use crate::AppState;

mod validations;

// String wrapper to allow derived impl of FromRow
#[derive(FromRow)]
pub struct Username(pub String);

// Verification code types
#[derive(Debug)]
pub enum CodeType {
    EmailVerification,
}

impl From<CodeType> for String {
    fn from(val: CodeType) -> Self {
        match val {
            CodeType::EmailVerification => "EmailVerification".to_string(),
        }
    }
}

// Wrapper for anyhow to allow impl of IntoResponse
pub struct AppError(anyhow::Error);

// Errors specific to our app
#[derive(Error, Debug)]
pub enum ErrorList {
    #[error("Email must contain an @, be greater than 3 characters and less than 300 characters")]
    InvalidEmail,
    #[error("Password must be between 8 and 100 characters")]
    InvalidPassword,
    #[error("Username must be between 3 and 100 characters")]
    InvalidUsername,
    #[error("Your passwords do not match")]
    NonMatchingPasswords,
    #[error("That email is already registered")]
    EmailAlreadyRegistered,
    #[error("That username is already registered")]
    UsernameAlreadyRegistered,
    #[error("Incorrect password")]
    IncorrectPassword,
    #[error("Incorrect username")]
    IncorrectUsername,
    #[error("Invalid or expired verification code")]
    InvalidVerificationCode,
    #[error("Unauthorised")]
    Unauthorised,
}

// Convert every AppError into a status code and its display impl
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal server error: {}", self.0),
        )
            .into_response()
    }
}

// Generic implementation to convert to AppError for anything which
// implements <Into anyhow:Error>
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Serialize, Deserialize)]
pub struct RegistrationDetails {
    username: String,
    email: String,
    password: String,
    confirm_password: String,
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct User {
    username: String,
    email: String,
    hashed_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginDetails {
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChangePassword {
    password: String,
    confirm_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerificationDetails {
    email: String,
    code: String,
}

pub async fn hello_world(headers: HeaderMap) -> Result<Html<String>, AppError> {
    println!("{:?}", headers);
    Ok(Html("Hello, what are you doing?".to_string()))
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(registration_details): Json<RegistrationDetails>,
) -> Result<Html<String>, AppError> {
    // Validate all the fields
    validate_email(&registration_details.email)?;
    validate_username(&registration_details.username)?;
    validate_password(&registration_details.password)?;
    is_unique(
        &registration_details.username,
        &registration_details.email,
        state.clone(),
    )
    .await?;
    if registration_details.password != registration_details.confirm_password {
        return Err(ErrorList::NonMatchingPasswords.into());
    }

    event!(
        Level::INFO,
        "Attempting to create registration for email {} and username {}",
        registration_details.email,
        registration_details.username
    );

    // Create a registration
    sqlx::query("INSERT INTO USERS(email,username,hashed_password) values(?,?,?)")
        .bind(&registration_details.email)
        .bind(&registration_details.username)
        .bind(hash_password(registration_details.password.as_str()))
        .execute(&state.db_connection_pool)
        .await?;

    event!(
        Level::INFO,
        "Attempting to send a verification email to {}",
        registration_details.email
    );

    // Send an email
    let to = format!(
        "{} <{}>",
        registration_details.username, registration_details.email
    );

    let code = generate_unique_id(8);

    let email = Email {
        to: to.as_str(),
        from: "registration@tld.com",
        subject: String::from("Verify your email"),
        body: format!(
            "<p>Thank you for registering.</p> <p>Please verify for your email using the following code {}.</p>",
            code
        ),
        reply_to: None,
    };
    add_code(
        state.clone(),
        &registration_details.email,
        &code,
        CodeType::EmailVerification,
    )
    .await?;
    //send_email(state.clone(), email).await?;

    Ok(Html("Registration successful".to_string()))
}

pub async fn add_code(
    state: Arc<AppState>,
    email: &String,
    code: &String,
    code_type: CodeType,
) -> Result<(), anyhow::Error> {
    let _created = sqlx::query(
        "INSERT INTO CODES(code_type,email,code,created_ts,expiry_ts) values(?,?,?,?,?)",
    )
    .bind(Into::<String>::into(code_type))
    .bind(email)
    .bind(code)
    .bind(Utc::now().timestamp())
    .bind(Utc::now().timestamp() + 24 * 3600)
    .execute(&state.db_connection_pool)
    .await?;
    Ok(())
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(login_details): Json<LoginDetails>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(login_details.email)
        .fetch_optional(&state.db_connection_pool)
        .await?;
    let user = match user {
        Some(i) => i,
        None => return Err(ErrorList::IncorrectUsername.into()),
    };
    let mut header_map = HeaderMap::new();
    if verify_password(&user.hashed_password, &login_details.password) {
        let session_key = generate_unique_id(100);
        let session_cookie = Cookie::build(("session-key", &session_key))
            .max_age(Duration::days(1000))
            .http_only(true)
            .build();
        header_map.insert(
            header::SET_COOKIE,
            session_cookie.to_string().parse().unwrap(),
        );
        let expiry = Utc::now().timestamp() + (1000 * 24 * 60 * 60);
        sqlx::query("INSERT INTO sessions(session_key,username, expiry) values(?, ?, ?)")
            .bind(session_key)
            .bind(user.username)
            .bind(expiry)
            .execute(&state.db_connection_pool)
            .await?;
        return Ok((header_map, Html("Login successful".to_string())));
    }
    Err(ErrorList::IncorrectPassword.into())
}

pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Json(verification_details): Json<VerificationDetails>,
) -> Result<Html<String>, AppError> {
    let now = Utc::now().timestamp();

    let code_exists = sqlx::query(
        "SELECT 1 FROM codes WHERE code_type = 'EmailVerification' AND email = ? AND code = ? AND expiry_ts > ?"
    )
    .bind(&verification_details.email)
    .bind(&verification_details.code)
    .bind(now)
    .fetch_optional(&state.db_connection_pool)
    .await?;

    if code_exists.is_none() {
        return Err(ErrorList::InvalidVerificationCode.into());
    }

    sqlx::query("UPDATE users SET auth_level = 50 WHERE email = ?")
        .bind(&verification_details.email)
        .execute(&state.db_connection_pool)
        .await?;

    // Clean up used code
    sqlx::query(
        "UPDATE codes SET used = true WHERE email = ? AND code=? AND code_type='EmailVerification'",
    )
    .bind(&verification_details.email)
    .bind(&verification_details.code)
    .execute(&state.db_connection_pool)
    .await?;

    Ok(Html("Email successfully verified".to_string()))
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Json(password_details): Json<ChangePassword>,
) -> Result<Html<String>, AppError> {
    validate_password(&password_details.password)?;

    if password_details.password != password_details.confirm_password {
        return Err(ErrorList::NonMatchingPasswords.into());
    }

    let hashed_password = hash_password(&password_details.password);

    sqlx::query("UPDATE users SET hashed_password = ? WHERE email = ?")
        .bind(hashed_password)
        .bind("example@email.com") // Would be replaced by authenticated user's email
        .execute(&state.db_connection_pool)
        .await?;

    Ok(Html("Password successfully changed".to_string()))
}

pub async fn reset_password() -> Result<Html<String>, StatusCode> {
    Ok(Html("Reset Password".to_string()))
}
