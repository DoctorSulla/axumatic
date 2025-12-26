use crate::{
    NONCE_STORE,
    auth::{
        IdentityProvider, add_code, create_registration, has_valid_email_code,
        send_verification_email,
    },
    user::{Profile, User, get_user_by_sub, get_user_by_username, update_google_user_email},
};
use axum::{
    async_trait,
    extract::{FromRequestParts, Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use cookie::Cookie;
use cookie::time::Duration;
use http::header::{self, HeaderMap, SET_COOKIE};
use jwt_verifier::JwtVerifierClient;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use thiserror::Error;
use tracing::{Level, event};
use validations::*;

use crate::{AppState, user::get_user_by_email};
use crate::{auth::create_session, utilities::*};

mod validations;

// Wrapper to allow derived impl of FromRow
#[derive(FromRow)]
pub struct UserEmail(pub String);

// Wrapper to allow derived impl of FromRow
#[derive(FromRow)]
pub struct CodeAndEmail(pub String, pub String);

#[derive(Serialize, Deserialize)]
pub struct PasswordResetInitiateRequest(pub String);

#[derive(Serialize, Deserialize)]
pub struct PasswordResetCompleteRequest {
    pub code: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct GoogleToken {
    jwt: String,
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
    #[error("Too many login attempts, please reset your password")]
    TooManyLoginAttempts,
    #[error("Unauthorised")]
    Unauthorised,
    #[error("Unexpected error verifying JWT")]
    UnexpectedJwtError,
    #[error("Invalid JWT")]
    InvalidJwt,
    #[error("Email is already registered with another identity provider")]
    EmailRegisteredWithAnotherProvider,
    #[error("User uses and Identity Provider rather than a password to authenticate")]
    UserDoesNotUsePassword,
    #[error("Your email is already verified")]
    EmailAlreadyVerified,
    #[error(
        "You must wait for your previous email verification code to expire before you can send another"
    )]
    PreviousCodeNotExpired,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse {
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
    UserProfile,
    Nonce,
    ResendVerificationEmailSuccess,
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
            ResponseType::UserProfile => "UserProfile".to_string(),
            ResponseType::Nonce => "Nonce".to_string(),
            ResponseType::ResendVerificationEmailSuccess => {
                "ResendVerificationEmailSuccess".to_string()
            }
        }
    }
}

// Convert every AppError into a status code and its display impl
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let message = format!("{}", self.0);
        let error_response = ApiResponse {
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
    pub sub: Option<String>,
}

// Used to extract the user from object from the username header
#[async_trait]
impl FromRequestParts<Arc<AppState>> for User {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let email = parts
            .headers
            .get("email")
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Expected header missing"))?;
        let email = email.to_str().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unexpected error with header value",
            )
        })?;
        let user = sqlx::query_as::<_, User>("select * from users where email=$1")
            .bind(email)
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
                ));
            }
        };

        Err((StatusCode::INTERNAL_SERVER_ERROR, "Error fetching user"))
    }
}

#[derive(Serialize, Deserialize)]
pub struct LoginDetails {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChangePassword {
    pub old_password: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct VerificationDetails {
    pub email: String,
    pub code: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(registration_details): Json<RegistrationDetails>,
) -> Result<Json<ApiResponse>, AppError> {
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

    let user = create_registration(
        &registration_details,
        state.clone(),
        IdentityProvider::Default,
    )
    .await?;

    send_verification_email(&user, state.clone()).await?;

    Ok(Json(ApiResponse {
        response_type: ResponseType::RegistrationSuccess,
        message: "Registration successful".to_string(),
    }))
}

pub async fn google_login(
    State(state): State<Arc<AppState>>,
    Json(token): Json<GoogleToken>,
) -> Result<(HeaderMap, Json<ApiResponse>), AppError> {
    let mut headers = HeaderMap::new();

    let jwt = token.jwt;

    event!(Level::INFO, "Verifying JWT");

    let mut client = JwtVerifierClient::new()
        .await
        .map_err(|_| AppError(ErrorList::UnexpectedJwtError.into()))?;

    let claims = JwtVerifierClient::verify(
        &mut client,
        &jwt,
        true,
        &state.config.server.google_client_id,
    )
    .await
    .map_err(|_| AppError(ErrorList::InvalidJwt.into()))?;

    let nonce = claims.nonce.ok_or(ErrorList::InvalidJwt)?;
    {
        let mut lock = NONCE_STORE.write().expect("Couldn't get lock");
        if lock.remove(&nonce).is_none() {
            return Err(AppError(ErrorList::InvalidJwt.into()));
        }
    }

    event!(Level::INFO, "JWT verified successfully");

    if let (Some(email), Some(verified)) = (claims.email, claims.email_verified) {
        let user = get_user_by_sub(state.clone(), &claims.sub).await;

        if let Ok(mut user) = user {
            event!(
                Level::INFO,
                "User is registered, checking email hasn't changed"
            );

            if email != user.email {
                // Check if new email is already registered
                if get_user_by_email(state.clone(), &user.email).await.is_ok() {
                    return Err(AppError(
                        ErrorList::EmailRegisteredWithAnotherProvider.into(),
                    ));
                } else {
                    // Update the user's email and email verification status
                    update_google_user_email(state.clone(), &email, verified, &claims.sub).await?;
                    user.email = email;
                    user.email_verified = verified;
                }
            }
            // Check registration type
            if user.identity_provider == "google" {
                event!(Level::INFO, "Registered with Google, creating session");
                let session_cookie = create_session(&user, state).await?;
                headers.insert(SET_COOKIE, session_cookie.to_string().parse()?);
            } else {
                event!(
                    Level::INFO,
                    "Registered with another provider, returning an error"
                );
                return Err(AppError(
                    ErrorList::EmailRegisteredWithAnotherProvider.into(),
                ));
            }
        } else {
            let (proposed_username, _prefix) =
                email.split_once('@').expect("Email does not contain an @");
            let username = match get_user_by_username(state.clone(), proposed_username).await {
                Ok(_v) => generate_unique_id(20),
                Err(_e) => proposed_username.to_string(),
            };
            let registration_details = RegistrationDetails {
                username,
                email,
                password: String::new(),
                confirm_password: String::new(),
                sub: Some(claims.sub),
            };
            if verified {
                //Create new reg with email verified
                create_registration(
                    &registration_details,
                    state.clone(),
                    IdentityProvider::Google,
                )
                .await?;

                sqlx::query("UPDATE users SET email_verified = true WHERE email = $1")
                    .bind(&registration_details.email)
                    .execute(&state.db_connection_pool)
                    .await?;

                let user = get_user_by_email(state.clone(), &registration_details.email).await?;
                let session_cookie = create_session(&user, state.clone()).await?;
                headers.insert(SET_COOKIE, session_cookie.to_string().parse()?);
            } else {
                // Create new unverified reg and send email
                create_registration(
                    &registration_details,
                    state.clone(),
                    IdentityProvider::Google,
                )
                .await?;

                let user = get_user_by_email(state.clone(), &registration_details.email).await?;
                send_verification_email(&user, state.clone()).await?;
                let session_cookie = create_session(&user, state.clone()).await?;
                headers.insert(SET_COOKIE, session_cookie.to_string().parse()?);
            }
        }
    }

    Ok((
        headers,
        Json(ApiResponse {
            message: "Login successful".to_string(),
            response_type: ResponseType::LoginSuccess,
        }),
    ))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(login_details): Json<LoginDetails>,
) -> Result<(HeaderMap, Json<ApiResponse>), AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&login_details.email)
        .fetch_optional(&state.db_connection_pool)
        .await?;
    let user = user.ok_or(ErrorList::IncorrectUsername)?;

    if user.identity_provider != "default" {
        return Err(ErrorList::EmailRegisteredWithAnotherProvider.into());
    }

    if user.login_attempts >= state.config.server.max_unsuccessful_login_attempts {
        event!(Level::WARN, "Account locked due to too many login attempts");
        return Err(ErrorList::TooManyLoginAttempts.into());
    }
    let mut header_map = HeaderMap::new();
    if verify_password(
        user.hashed_password
            .as_ref()
            .expect("User missing password"),
        &login_details.password,
    ) {
        let session_cookie = create_session(&user, state.clone()).await?;

        sqlx::query("UPDATE users SET login_attempts=0 WHERE email=$1")
            .bind(&user.email)
            .execute(&state.db_connection_pool)
            .await?;

        header_map.insert(header::SET_COOKIE, session_cookie.to_string().parse()?);
        Ok((
            header_map,
            Json(ApiResponse {
                response_type: ResponseType::LoginSuccess,
                message: "Login successful".to_string(),
            }),
        ))
    } else {
        let _ = sqlx::query("UPDATE users SET login_attempts=$1 WHERE email=$2")
            .bind(user.login_attempts + 1)
            .bind(&login_details.email)
            .execute(&state.db_connection_pool)
            .await?;
        Err(ErrorList::IncorrectPassword.into())
    }
}

pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Json(verification_details): Json<VerificationDetails>,
) -> Result<Json<ApiResponse>, AppError> {
    let now = Utc::now().timestamp();

    let code_exists = sqlx::query(
        "SELECT 1 FROM codes WHERE code_type = 'EmailVerification' AND email = $1 AND code = $2 AND expiry_ts > $3"
    )
    .bind(&verification_details.email)
    .bind(&verification_details.code)
    .bind(now)
    .fetch_optional(&state.db_connection_pool)
    .await?;

    if code_exists.is_none() {
        return Err(ErrorList::InvalidVerificationCode.into());
    }

    sqlx::query("UPDATE users SET email_verified = true WHERE email = $1")
        .bind(&verification_details.email)
        .execute(&state.db_connection_pool)
        .await?;

    // Clean up used code
    sqlx::query(
        "UPDATE codes SET used = true WHERE email = $1 AND code=$2 AND code_type='EmailVerification'",
    )
    .bind(&verification_details.email)
    .bind(&verification_details.code)
    .execute(&state.db_connection_pool)
    .await?;

    Ok(Json(ApiResponse {
        message: "Email verified successfully".to_string(),
        response_type: ResponseType::EmailVerificationSuccess,
    }))
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(password_details): Json<ChangePassword>,
) -> Result<Json<ApiResponse>, AppError> {
    if user.identity_provider != *"default" {
        return Err(ErrorList::UserDoesNotUsePassword.into());
    }
    if !verify_password(
        user.hashed_password
            .as_ref()
            .expect("User missing password"),
        &password_details.old_password,
    ) {
        return Err(ErrorList::IncorrectPassword.into());
    }
    validate_password(&password_details.password)?;

    if password_details.password != password_details.confirm_password {
        return Err(ErrorList::NonMatchingPasswords.into());
    }

    let hashed_password = hash_password(&password_details.password);

    sqlx::query("UPDATE users SET hashed_password = $1 WHERE email = $2")
        .bind(hashed_password)
        .bind(user.email)
        .execute(&state.db_connection_pool)
        .await?;

    Ok(Json(ApiResponse {
        message: "Password changed successfully".to_string(),
        response_type: ResponseType::PasswordChangeSuccess,
    }))
}

pub async fn password_reset_initiate(
    State(state): State<Arc<AppState>>,
    Json(password_reset_request): Json<PasswordResetInitiateRequest>,
) -> Result<Json<ApiResponse>, AppError> {
    // Check if user exists for provided email
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&password_reset_request.0)
        .fetch_optional(&state.db_connection_pool)
        .await?;

    let user = user.ok_or(ErrorList::IncorrectUsername)?;

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
            <p>Use this code to reset your password: {code}</p> \
            <p>If you did not request this, please ignore this email.</p>"
        ),
        reply_to: None,
    };

    send_email(state, email).await?;

    Ok(Json(ApiResponse {
        message: "Password reset email sent".to_string(),
        response_type: ResponseType::PasswordResetInitiationSuccess,
    }))
}

pub async fn password_reset_complete(
    State(state): State<Arc<AppState>>,
    Json(password_reset_response): Json<PasswordResetCompleteRequest>,
) -> Result<Json<ApiResponse>, AppError> {
    // Check if passwords match
    if password_reset_response.password != password_reset_response.confirm_password {
        return Err(ErrorList::NonMatchingPasswords.into());
    }

    // Check if code is valid
    let code = sqlx::query_as::<_,CodeAndEmail>("SELECT code,email FROM codes WHERE code_type='PasswordReset' AND used=false AND expiry_ts > $1 AND code=$2")
            .bind(Utc::now().timestamp())
                    .bind(password_reset_response.code).fetch_optional(&state.db_connection_pool).await?;

    if let Some(code) = code {
        // Update password
        sqlx::query("UPDATE users SET hashed_password=$1, login_attempts=0 WHERE email=$2")
            .bind(hash_password(password_reset_response.password.as_str()))
            .bind(code.1)
            .execute(&state.db_connection_pool)
            .await?;
        // Mark code as used
        sqlx::query("UPDATE codes SET used=true WHERE code=$1")
            .bind(code.0)
            .execute(&state.db_connection_pool)
            .await?;
    } else {
        return Err(ErrorList::InvalidVerificationCode.into());
    }

    Ok(Json(ApiResponse {
        message: "Password reset complete".to_string(),
        response_type: ResponseType::PasswordResetSuccess,
    }))
}

pub async fn get_profile(user: User) -> Result<Json<ApiResponse>, AppError> {
    let profile = Profile::from(user);

    Ok(Json(ApiResponse {
        response_type: ResponseType::UserProfile,
        message: serde_json::to_string(&profile).expect("Could not convert profile to string"),
    }))
}

pub async fn logout(State(state): State<Arc<AppState>>, user: User) -> Result<HeaderMap, AppError> {
    sqlx::query("DELETE FROM sessions WHERE username=$1")
        .bind(&user.username)
        .execute(&state.db_connection_pool)
        .await?;

    let logout_cookie = Cookie::build(("session-key", ""))
        .max_age(Duration::days(-state.config.server.session_length_in_days))
        .path("/")
        .secure(true)
        .http_only(true)
        .build();

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, logout_cookie.to_string().parse()?);
    Ok(headers)
}

pub async fn resend_verification_email(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<ApiResponse>, AppError> {
    if user.email_verified {
        Err(AppError(ErrorList::EmailAlreadyVerified.into()))
    } else if has_valid_email_code(state.clone(), &user).await.is_some() {
        Err(AppError(ErrorList::PreviousCodeNotExpired.into()))
    } else {
        send_verification_email(&user, state.clone()).await?;
        Ok(Json(ApiResponse {
            response_type: ResponseType::ResendVerificationEmailSuccess,
            message: "Verification email sent successfully".to_string(),
        }))
    }
}

pub async fn health_check() -> http::status::StatusCode {
    http::status::StatusCode::NO_CONTENT
}

pub async fn get_nonce() -> Result<Json<ApiResponse>, AppError> {
    const NONCE_EXPIRATION: i64 = 300;

    let mut lock = NONCE_STORE.write().expect("Couldn't acquire lock");
    let id = generate_unique_id(20);

    let now = Utc::now().timestamp();

    lock.retain(|_k, v| *v + NONCE_EXPIRATION > now);

    lock.insert(id.clone(), now);

    Ok(Json(ApiResponse {
        response_type: ResponseType::Nonce,
        message: id,
    }))
}
