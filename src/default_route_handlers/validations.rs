use crate::AppState;
use std::sync::Arc;
use tracing::{Level, event};

use super::ErrorList;

pub fn validate_email(email: &str) -> Result<bool, ErrorList> {
    if email.contains('@') && email.len() > 3 {
        return Ok(true);
    }
    Err(ErrorList::InvalidEmail)
}

pub fn validate_password(password: &str) -> Result<bool, ErrorList> {
    if password.len() >= 8 && password.len() < 100 {
        return Ok(true);
    }
    Err(ErrorList::InvalidPassword)
}

pub fn validate_username(username: &str) -> Result<bool, ErrorList> {
    if username.len() >= 3 && username.len() < 100 {
        return Ok(true);
    }
    Err(ErrorList::InvalidUsername)
}

pub async fn is_unique(
    username: &String,
    email: &String,
    state: Arc<AppState>,
) -> Result<bool, ErrorList> {
    event!(
        Level::INFO,
        "Checking if username of {} or email of {} is registered",
        &username,
        &email
    );

    let username_exists = sqlx::query!(
        "SELECT 1 as exists FROM users WHERE username = $1",
        username
    )
    .fetch_optional(&state.db_connection_pool)
    .await;

    if let Ok(Some(_)) = username_exists {
        event!(
            Level::INFO,
            "Attempted registration with duplicate username"
        );
        return Err(ErrorList::UsernameAlreadyRegistered);
    }

    let email_exists = sqlx::query!("SELECT email FROM users WHERE email = $1", email)
        .fetch_optional(&state.db_connection_pool)
        .await;

    if let Ok(Some(_)) = email_exists {
        event!(Level::INFO, "Attempted registration with duplicate email");
        return Err(ErrorList::EmailAlreadyRegistered);
    }
    Ok(true)
}
