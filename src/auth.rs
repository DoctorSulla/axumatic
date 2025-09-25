use crate::AppState;
use crate::default_route_handlers::User;
use crate::default_route_handlers::{AppError, ErrorList, Username};
use crate::utilities::generate_unique_id;
use chrono::Utc;
use cookie::Cookie;
use cookie::time::Duration;
use http::HeaderMap;
use std::sync::Arc;
use tracing::{Level, event};

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

pub async fn create_session(user: &User, state: Arc<AppState>) -> Result<Cookie, AppError> {
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
