use crate::route_handlers::{ErrorList, Username};
use crate::AppState;
use chrono::Utc;
use cookie::Cookie;
use http::HeaderMap;
use std::sync::Arc;
use tracing::{event, Level};

pub async fn validate_cookie(
    headers: &HeaderMap,
    state: Arc<AppState>,
) -> Result<Username, anyhow::Error> {
    if let Some(cookies) = headers.get("cookie") {
        for cookie_string in cookies.to_str().unwrap().split(';') {
            let cookie = Cookie::parse(cookie_string)?;
            if cookie.name() == "session-key" {
                let session = sqlx::query_as::<_, Username>(
                    "SELECT username FROM SESSIONS WHERE session_key=? AND expiry > ?",
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
