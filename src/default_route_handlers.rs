use axum::{
    async_trait,
    extract::{FromRequestParts, Json, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use chrono::Utc;
use cookie::{time::Duration, Cookie};
use http::{header, header::HeaderMap};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use thiserror::Error;
use tracing::{event, Level};
use validations::*;

use crate::utilities::*;
use crate::AppState;

mod validations;

// Wrapper to allow derived impl of FromRow
#[derive(FromRow)]
pub struct Username(pub String);

// Wrapper to allow derived impl of FromRow
#[derive(FromRow)]
pub struct CodeAndEmail(pub String, pub String);

#[derive(Deserialize)]
pub struct PasswordResetInitiateRequest(pub String);

#[derive(Deserialize)]
pub struct PasswordResetCompleteRequest {
    pub code: String,
    pub password: String,
    pub confirm_password: String,
}

// Verification code types
#[derive(Debug)]
pub enum CodeType {
    EmailVerification,
    PasswordReset,
}

impl From<CodeType> for String {
    fn from(val: CodeType) -> Self {
        match val {
            CodeType::EmailVerification => "EmailVerification".to_string(),
            CodeType::PasswordReset => "PasswordReset".to_string(),
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

#[derive(Serialize, Deserialize)]
pub struct AuthAndLoginResponse {
    pub response_type: ResponseType,
    pub message: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ResponseType {
    Error,
    RegistrationSuccess,
    LoginSuccess,
    EmailVerificationSuccess,
    PasswordChangeSuccess,
    PasswordResetInitiationSuccess,
    PasswordResetSuccess,
}

impl From<ResponseType> for String {
    fn from(value: ResponseType) -> Self {
        match value {
            ResponseType::Error => "Error".to_string(),
            ResponseType::LoginSuccess => "LoginSuccess".to_string(),
            ResponseType::RegistrationSuccess => "RegistrationSuccess".to_string(),
            ResponseType::EmailVerificationSuccess => "EmailVerificationSuccess".to_string(),
            ResponseType::PasswordChangeSuccess => "PasswordChangeSuccess".to_string(),
            ResponseType::PasswordResetInitiationSuccess => {
                "PasswordResetInitiationSuccess".to_string()
            }
            ResponseType::PasswordResetSuccess => "PasswordResetSuccess".to_string(),
        }
    }
}

// Convert every AppError into a status code and its display impl
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let message = format!("{}", self.0);
        let error_response = AuthAndLoginResponse {
            message,
            response_type: ResponseType::Error,
        };
        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
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
    pub username: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct User {
    username: String,
    email: String,
    hashed_password: String,
    auth_level: String,
}

// Used to extract the user from object from the username header
#[async_trait]
impl FromRequestParts<Arc<AppState>> for User {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let username = match parts.headers.get("username") {
            Some(username) => username,
            None => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Expected header missing")),
        };
        let username = match username.to_str() {
            Ok(i) => i,
            Err(_e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unexpected error with header value",
                ))
            }
        };
        let user = sqlx::query_as::<_, User>("select * from users where username=?")
            .bind(username)
            .fetch_optional(&state.db_connection_pool)
            .await;

        match user {
            Ok(user) => {
                if let Some(user) = user {
                    return Ok(user);
                }
            }
            Err(_e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unexpected error fetching user",
                ))
            }
        };
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Error fetching user"))
        //Ok(user.unwrap().unwrap())

        // Ok(User {
        //     username: "test".to_string(),
        //     email: "blah".to_string(),
        //     hashed_password: "hmm".to_string(),
        // })
    }
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

pub async fn hello_world(user: User) -> Result<Html<String>, AppError> {
    println!("The authenticated user is {:?}", user);
    Ok(Html("Hello, what are you doing?".to_string()))
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(registration_details): Json<RegistrationDetails>,
) -> Result<Json<AuthAndLoginResponse>, AppError> {
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
    send_email(state.clone(), email).await?;

    return Ok(Json(AuthAndLoginResponse {
        response_type: ResponseType::RegistrationSuccess,
        message: "Registration successful".to_string(),
    }));
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
) -> Result<(HeaderMap, Json<AuthAndLoginResponse>), AppError> {
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
            .path("/")
            .secure(true)
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
        return Ok((
            header_map,
            Json(AuthAndLoginResponse {
                response_type: ResponseType::LoginSuccess,
                message: "Login successful".to_string(),
            }),
        ));
    }
    Err(ErrorList::IncorrectPassword.into())
}

pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Json(verification_details): Json<VerificationDetails>,
) -> Result<Json<AuthAndLoginResponse>, AppError> {
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

    sqlx::query("UPDATE users SET auth_level = 'verified' WHERE email = ?")
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

    Ok(Json(AuthAndLoginResponse {
        message: "Email verified successfully".to_string(),
        response_type: ResponseType::EmailVerificationSuccess,
    }))
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(password_details): Json<ChangePassword>,
) -> Result<Json<AuthAndLoginResponse>, AppError> {
    validate_password(&password_details.password)?;

    if password_details.password != password_details.confirm_password {
        return Err(ErrorList::NonMatchingPasswords.into());
    }

    let hashed_password = hash_password(&password_details.password);

    sqlx::query("UPDATE users SET hashed_password = ? WHERE email = ?")
        .bind(hashed_password)
        .bind(user.email)
        .execute(&state.db_connection_pool)
        .await?;

    Ok(Json(AuthAndLoginResponse {
        message: "Password changed successfully".to_string(),
        response_type: ResponseType::PasswordChangeSuccess,
    }))
}

pub async fn password_reset_initiate(
    State(state): State<Arc<AppState>>,
    Json(password_reset_request): Json<PasswordResetInitiateRequest>,
) -> Result<Json<AuthAndLoginResponse>, AppError> {
    // Check if user exists for provided email
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&password_reset_request.0)
        .fetch_optional(&state.db_connection_pool)
        .await?;

    let user = match user {
        Some(u) => u,
        None => return Err(ErrorList::IncorrectUsername.into()),
    };

    // Generate a code
    let code = generate_unique_id(8);

    // Add code to database
    add_code(state.clone(), &user.email, &code, CodeType::PasswordReset).await?;

    // Send email
    let email = Email {
        to: &user.email,
        from: "registration@tld.com",
        subject: String::from("Password Reset"),
        body: format!(
            "<p>A password reset was requested for your account.</p> \
            <p>Use this code to reset your password: {}</p> \
            <p>If you did not request this, please ignore this email.</p>",
            code
        ),
        reply_to: None,
    };

    send_email(state, email).await?;

    Ok(Json(AuthAndLoginResponse {
        message: "Password reset email sent".to_string(),
        response_type: ResponseType::PasswordResetInitiationSuccess,
    }))
}

pub async fn password_reset_complete(
    State(state): State<Arc<AppState>>,
    Json(password_reset_response): Json<PasswordResetCompleteRequest>,
) -> Result<Json<AuthAndLoginResponse>, AppError> {
    // Check if passwords match
    if password_reset_response.password != password_reset_response.confirm_password {
        return Err(ErrorList::NonMatchingPasswords.into());
    }

    // Check if code is valid
    let code = sqlx::query_as::<_,CodeAndEmail>("SELECT code,email FROM codes WHERE code_type='PasswordReset' AND used=0 AND expiry_ts > ? AND code=?")
            .bind(Utc::now().timestamp())
                    .bind(password_reset_response.code).fetch_optional(&state.db_connection_pool).await?;

    if let Some(code) = code {
        // Update password
        sqlx::query("UPDATE users SET hashed_password=? WHERE email=?")
            .bind(hash_password(password_reset_response.password.as_str()))
            .bind(code.1)
            .execute(&state.db_connection_pool)
            .await?;
        // Mark code as used
        sqlx::query("UPDATE codes SET used=1 WHERE code=?")
            .bind(code.0)
            .execute(&state.db_connection_pool)
            .await?;
    } else {
        return Err(ErrorList::InvalidVerificationCode.into());
    }

    Ok(Json(AuthAndLoginResponse {
        message: "Password reset complete".to_string(),
        response_type: ResponseType::PasswordResetSuccess,
    }))
}
