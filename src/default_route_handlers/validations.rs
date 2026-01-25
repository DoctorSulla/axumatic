use crate::AppState;
use std::sync::Arc;
use tracing::{Level, event};

use super::ErrorList;

// Validation constants
const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_PASSWORD_LENGTH: usize = 100;
const MIN_USERNAME_LENGTH: usize = 3;
const MAX_USERNAME_LENGTH: usize = 100;
const MIN_EMAIL_LENGTH: usize = 3;

pub fn validate_email(email: &str) -> Result<bool, ErrorList> {
    if email.contains('@') && email.len() >= MIN_EMAIL_LENGTH {
        return Ok(true);
    }
    Err(ErrorList::InvalidEmail)
}

pub fn validate_password(password: &str) -> Result<bool, ErrorList> {
    if password.len() >= MIN_PASSWORD_LENGTH && password.len() <= MAX_PASSWORD_LENGTH {
        return Ok(true);
    }
    Err(ErrorList::InvalidPassword)
}

pub fn validate_username(username: &str) -> Result<bool, ErrorList> {
    if username.len() >= MIN_USERNAME_LENGTH && username.len() <= MAX_USERNAME_LENGTH {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_length_password() {
        assert!(validate_password("ABCDABCD").is_ok());
    }

    #[test]
    fn too_short_password() {
        assert!(validate_password("ABCDABC").is_err());
    }

    #[test]
    fn max_length_password() {
        assert!(validate_password("ABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRST").is_ok());
    }

    #[test]
    fn too_long_password() {
        assert!(validate_password("ABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRSTABCDEFGHIJKLMNOPQRSTA").is_err());
    }

    // Email validation tests
    #[test]
    fn valid_email() {
        assert!(validate_email("test@example.com").is_ok());
    }

    #[test]
    fn email_with_at_symbol() {
        assert!(validate_email("a@b").is_ok());
    }

    #[test]
    fn email_without_at_symbol() {
        assert!(validate_email("invalid.email.com").is_err());
    }

    #[test]
    fn email_too_short() {
        assert!(validate_email("ab").is_err());
    }

    #[test]
    fn empty_email() {
        assert!(validate_email("").is_err());
    }

    // Username validation tests
    #[test]
    fn min_length_username() {
        assert!(validate_username("abc").is_ok());
    }

    #[test]
    fn too_short_username() {
        assert!(validate_username("ab").is_err());
    }

    #[test]
    fn max_length_username() {
        let long_username = "a".repeat(MAX_USERNAME_LENGTH);
        assert!(validate_username(&long_username).is_ok());
    }

    #[test]
    fn too_long_username() {
        let too_long_username = "a".repeat(MAX_USERNAME_LENGTH + 1);
        assert!(validate_username(&too_long_username).is_err());
    }

    #[test]
    fn empty_username() {
        assert!(validate_username("").is_err());
    }
}
