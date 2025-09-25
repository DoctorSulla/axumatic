use crate::AppState;
use std::sync::Arc;

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
    let username = sqlx::query("SELECT username FROM users WHERE username=?")
        .bind(username)
        .fetch_optional(&state.db_connection_pool)
        .await;

    if let Ok(user) = username {
        if user.is_some() {
            return Err(ErrorList::UsernameAlreadyRegistered);
        }
    }

    let email = sqlx::query("SELECT email FROM users WHERE email=?")
        .bind(email)
        .fetch_optional(&state.db_connection_pool)
        .await;

    if let Ok(email) = email {
        if email.is_some() {
            return Err(ErrorList::EmailAlreadyRegistered);
        }
    }
    Ok(true)
}

pub async fn is_email_registered(email: &String, state: Arc<AppState>) -> Result<bool, ErrorList> {
    let email = sqlx::query("SELECT email FROM users WHERE email=?")
        .bind(email)
        .fetch_optional(&state.db_connection_pool)
        .await;

    if let Ok(email) = email {
        if email.is_some() {
            return Err(ErrorList::EmailAlreadyRegistered);
        }
    }
    Ok(true)
}
