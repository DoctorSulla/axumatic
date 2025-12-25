use crate::AppState;
use crate::config::AuthLevel;
use crate::default_route_handlers::{
    AppError, CodeType, ErrorList, RegistrationDetails, UserEmail,
};
use crate::user::User;
use crate::utilities::{Email, generate_unique_id, hash_password, send_email};
use chrono::Utc;
use cookie::Cookie;
use cookie::time::Duration;
use http::HeaderMap;
use sqlx::postgres::PgRow;
use std::sync::Arc;
use tracing::{Level, event};

const HOURS_IN_DAY: u32 = 24;
const SECONDS_IN_HOUR: u32 = 3600;

#[derive(Clone)]
pub enum IdentityProvider {
    Google,
    Default,
}

impl From<String> for IdentityProvider {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "default" => Self::Default,
            "google" => Self::Google,
            _ => Self::Default,
        }
    }
}

impl From<IdentityProvider> for String {
    fn from(value: IdentityProvider) -> Self {
        match value {
            IdentityProvider::Google => "google".to_string(),
            IdentityProvider::Default => "default".to_string(),
        }
    }
}

pub async fn validate_cookie(
    headers: &HeaderMap,
    state: Arc<AppState>,
) -> Result<UserEmail, anyhow::Error> {
    if let Some(cookies) = headers.get("cookie") {
        for cookie_string in cookies.to_str()?.split(';') {
            let cookie = Cookie::parse(cookie_string)?;
            if cookie.name() == "session-key" {
                let session = sqlx::query_as::<_, UserEmail>(
                    "SELECT email FROM SESSIONS WHERE session_key=$1 AND expiry > $2",
                )
                .bind(cookie.value())
                .bind(Utc::now().timestamp())
                .fetch_optional(&state.db_connection_pool)
                .await?;
                if let Some(username) = session {
                    return Ok(username);
                }
                event!(
                    Level::INFO,
                    "Session key cookie was found but did not match a valid session"
                );
                return Err(ErrorList::Unauthorised.into());
            }
        }
    }

    event!(Level::INFO, "No session key cookie was found");
    Err(ErrorList::Unauthorised.into())
}

pub async fn create_session(user: &User, state: Arc<AppState>) -> Result<Cookie<'_>, AppError> {
    let session_key = generate_unique_id(100);
    let session_cookie = Cookie::build(("session-key", session_key.clone()))
        .max_age(Duration::days(state.config.server.session_length_in_days))
        .path("/")
        .secure(true)
        .http_only(true)
        .build();

    let expiry = Utc::now().timestamp()
        + (state.config.server.session_length_in_days
            * HOURS_IN_DAY as i64
            * SECONDS_IN_HOUR as i64);

    sqlx::query("INSERT INTO sessions(session_key,email, expiry) values($1,$2,$3)")
        .bind(&session_key)
        .bind(&user.email)
        .bind(expiry)
        .execute(&state.db_connection_pool)
        .await?;

    Ok(session_cookie)
}

pub async fn create_registration(
    registration_details: &RegistrationDetails,
    state: Arc<AppState>,
    identity_provider: IdentityProvider,
) -> Result<User, AppError> {
    event!(
        Level::INFO,
        "Attempting to create registration for email {} and username {}",
        registration_details.email,
        registration_details.username
    );

    let hashed_password = hash_password(&registration_details.password);
    let registration_ts = Utc::now().timestamp();

    match identity_provider {
        IdentityProvider::Google => sqlx::query(
            "INSERT INTO USERS(email,username,registration_ts,identity_provider,sub) values($1,$2,$3,$4,$5)"
        )
    .bind(&registration_details.email)
    .bind(&registration_details.username)
    .bind(registration_ts)
    .bind(String::from(identity_provider.clone()))
    .bind(registration_details.sub.as_ref().expect("Sub missing for Google registration"))
    .execute(&state.db_connection_pool)
    .await?,
        IdentityProvider::Default => sqlx::query(
            "INSERT INTO USERS(email,username,hashed_password,registration_ts,identity_provider) values($1,$2,$3,$4,$5)",
        )
    .bind(&registration_details.email)
    .bind(&registration_details.username)
    .bind(&hashed_password)
    .bind(registration_ts)
    .bind(String::from(identity_provider.clone()))
    .execute(&state.db_connection_pool).await?
    };
    Ok(User {
        username: registration_details.username.clone(),
        email: registration_details.email.clone(),
        email_verified: false,
        hashed_password: None,
        auth_level: String::from(AuthLevel::User),
        login_attempts: 0,
        registration_ts,
        identity_provider: String::from(identity_provider),
    })
}

pub async fn send_verification_email(user: &User, state: Arc<AppState>) -> Result<(), AppError> {
    event!(
        Level::INFO,
        "Attempting to send a verification email to {}",
        user.email
    );

    // Send an email
    let to = format!("{} <{}>", user.username, user.email);

    let code = generate_unique_id(8);

    let email = Email {
        to: to.as_str(),
        from: "registration@tld.com",
        subject: String::from("Verify your email"),
        body: format!(
            "<p>Thank you for registering.</p> <p>Please verify for your email using the following code {code}. Your code is valid for 1 hour.</p>"
        ),
        reply_to: None,
    };
    add_code(
        state.clone(),
        &user.email,
        &code,
        CodeType::EmailVerification,
    )
    .await?;
    send_email(state.clone(), email).await?;
    Ok(())
}

pub async fn add_code(
    state: Arc<AppState>,
    email: &String,
    code: &String,
    code_type: CodeType,
) -> Result<(), anyhow::Error> {
    let _created = sqlx::query(
        "INSERT INTO CODES(code_type,email,code,created_ts,expiry_ts) values($1,$2,$3,$4,$5)",
    )
    .bind(Into::<String>::into(code_type))
    .bind(email)
    .bind(code)
    .bind(Utc::now().timestamp())
    .bind(Utc::now().timestamp() + SECONDS_IN_HOUR as i64)
    .execute(&state.db_connection_pool)
    .await?;
    Ok(())
}

pub async fn has_valid_email_code(state: Arc<AppState>, user: &User) -> Option<PgRow> {
    let now = Utc::now().timestamp();

    sqlx::query(
        "SELECT 1 FROM codes WHERE code_type = 'EmailVerification' AND email = $1 AND expiry_ts > $2"
    )
    .bind(&user.email)
    .bind(now)
    .fetch_optional(&state.db_connection_pool)
    .await
    .ok()
    .flatten()
}
