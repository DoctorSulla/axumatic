use crate::AppState;
use crate::default_route_handlers::{AppError, CodeType, ErrorList, RegistrationDetails, Username};
use crate::user::User;
use crate::utilities::{Email, generate_unique_id, hash_password, send_email};
use chrono::Utc;
use cookie::Cookie;
use cookie::time::Duration;
use http::HeaderMap;
use std::sync::Arc;
use tracing::{Level, event};

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
) -> Result<Username, anyhow::Error> {
    if let Some(cookies) = headers.get("cookie") {
        for cookie_string in cookies.to_str()?.split(';') {
            let cookie = Cookie::parse(cookie_string)?;
            if cookie.name() == "session-key" {
                let session = sqlx::query_as::<_, Username>(
                    "SELECT username FROM SESSIONS WHERE session_key=$1 AND expiry > $2",
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
        .max_age(Duration::days(1000))
        .path("/")
        .secure(true)
        .http_only(true)
        .build();

    let expiry = Utc::now().timestamp() + (1000 * 24 * 60 * 60);

    sqlx::query("INSERT INTO sessions(session_key,username, expiry) values($1,$2,$3)")
        .bind(&session_key)
        .bind(&user.username)
        .bind(expiry)
        .execute(&state.db_connection_pool)
        .await?;

    Ok(session_cookie)
}

pub async fn create_registration(
    registration_details: &RegistrationDetails,
    state: Arc<AppState>,
    identity_provider: IdentityProvider,
) -> Result<(), AppError> {
    event!(
        Level::INFO,
        "Attempting to create registration for email {} and username {}",
        registration_details.email,
        registration_details.username
    );

    match identity_provider {
        IdentityProvider::Google => sqlx::query(
            "INSERT INTO USERS(email,username,registration_ts,identity_provider,sub) values($1,$2,$3,$4,$5)"
        )
    .bind(&registration_details.email)
    .bind(&registration_details.username)
    .bind(Utc::now().timestamp())
    .bind(String::from(identity_provider))
    .bind(registration_details.sub.as_ref().expect("Sub missing for Google registration"))
    .execute(&state.db_connection_pool)
    .await?,
        IdentityProvider::Default => sqlx::query(
            "INSERT INTO USERS(email,username,hashed_password,registration_ts,identity_provider) values($1,$2,$3,$4,$5)",
        )
    .bind(&registration_details.email)
    .bind(&registration_details.username)
    .bind(hash_password(registration_details.password.as_str()))
    .bind(Utc::now().timestamp())
    .bind(String::from(identity_provider))
    .execute(&state.db_connection_pool).await?
    };
    Ok(())
}

pub async fn send_verification_email(
    registration_details: &RegistrationDetails,
    state: Arc<AppState>,
) -> Result<(), AppError> {
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
            "<p>Thank you for registering.</p> <p>Please verify for your email using the following code {code}.</p>"
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
    .bind(Utc::now().timestamp() + 24 * 3600)
    .execute(&state.db_connection_pool)
    .await?;
    Ok(())
}
