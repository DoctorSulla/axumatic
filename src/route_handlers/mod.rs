use axum::extract::{Json, State};
use axum::response::IntoResponse;
use axum::{http::StatusCode, response::Html};
use http::header;
use http::header::HeaderMap;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use thiserror::Error;
use validations::*;

use crate::utilities::{generate_unique_id, verify_password};
use crate::AppState;

mod validations;

pub struct AppError(anyhow::Error);

// Use this enum for errors specific to our app
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

pub async fn hello_world() -> Result<Html<String>, AppError> {
    Ok(Html("Hello World".to_string()))
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(registration_details): Json<RegistrationDetails>,
) -> Result<Html<String>, AppError> {
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

    sqlx::query("INSERT INTO USERS(email,username,hashed_password) values(?,?,?)")
        .bind(registration_details.email)
        .bind(registration_details.username)
        .bind(crate::utilities::hash_password(
            registration_details.password.as_str(),
        ))
        .execute(&state.connection_pool)
        .await?;

    Ok(Html("Registration successful".to_string()))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(login_details): Json<LoginDetails>,
) -> Result<(HeaderMap, Html<String>), AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(login_details.email)
        .fetch_optional(&state.connection_pool)
        .await?;
    let user = match user {
        Some(i) => i,
        None => return Err(ErrorList::IncorrectUsername.into()),
    };
    let mut header_map = HeaderMap::new();
    if verify_password(&user.hashed_password, &login_details.password) {
        header_map.insert(
            header::SET_COOKIE,
            header::HeaderValue::from_str(
                format!(
                    "session-key={};HttpOnly;Max-Age=8640000",
                    generate_unique_id(100)
                )
                .as_str(),
            )?,
        );
        return Ok((header_map, Html("Login successful".to_string())));
    }
    return Err(ErrorList::IncorrectPassword.into());
}

pub async fn verify_email() -> Result<Html<String>, StatusCode> {
    Ok(Html("Verify Email".to_string()))
}

pub async fn change_password() -> Result<Html<String>, StatusCode> {
    Ok(Html("Change Password".to_string()))
}

pub async fn reset_password() -> Result<Html<String>, StatusCode> {
    Ok(Html("Reset Password".to_string()))
}
