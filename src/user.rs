use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::config::AppState;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct User {
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub hashed_password: Option<String>,
    pub auth_level: String,
    pub login_attempts: i32,
    pub registration_ts: i64,
    pub identity_provider: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Profile {
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub auth_level: String,
    pub identity_provider: String,
    pub registration_ts: i64,
}

impl From<User> for Profile {
    fn from(value: User) -> Self {
        Self {
            username: value.username,
            email: value.email,
            email_verified: value.email_verified,
            auth_level: value.auth_level,
            identity_provider: value.identity_provider,
            registration_ts: value.registration_ts,
        }
    }
}

pub async fn get_user_by_email(state: Arc<AppState>, email: &str) -> Result<User, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT 
            username as "username!", 
            email as "email!", 
            email_verified as "email_verified!", 
            hashed_password, 
            auth_level as "auth_level!", 
            login_attempts as "login_attempts!", 
            registration_ts as "registration_ts!", 
            identity_provider as "identity_provider!" 
        FROM users WHERE email = $1"#,
        email
    )
    .fetch_optional(&state.db_connection_pool)
    .await?;

    match row {
        Some(r) => Ok(User {
            username: r.username,
            email: r.email,
            email_verified: r.email_verified,
            hashed_password: r.hashed_password,
            auth_level: r.auth_level,
            login_attempts: r.login_attempts,
            registration_ts: r.registration_ts,
            identity_provider: r.identity_provider,
        }),
        None => Err(anyhow!("User not found")),
    }
}

pub async fn get_user_by_sub(state: Arc<AppState>, sub: &str) -> Result<User, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT 
            username as "username!", 
            email as "email!", 
            email_verified as "email_verified!", 
            hashed_password, 
            auth_level as "auth_level!", 
            login_attempts as "login_attempts!", 
            registration_ts as "registration_ts!", 
            identity_provider as "identity_provider!" 
        FROM users WHERE sub = $1"#,
        sub
    )
    .fetch_optional(&state.db_connection_pool)
    .await?;

    match row {
        Some(r) => Ok(User {
            username: r.username,
            email: r.email,
            email_verified: r.email_verified,
            hashed_password: r.hashed_password,
            auth_level: r.auth_level,
            login_attempts: r.login_attempts,
            registration_ts: r.registration_ts,
            identity_provider: r.identity_provider,
        }),
        None => Err(anyhow!("User not found")),
    }
}

pub async fn get_user_by_username(
    state: Arc<AppState>,
    username: &str,
) -> Result<User, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT 
            username as "username!", 
            email as "email!", 
            email_verified as "email_verified!", 
            hashed_password, 
            auth_level as "auth_level!", 
            login_attempts as "login_attempts!", 
            registration_ts as "registration_ts!", 
            identity_provider as "identity_provider!" 
        FROM users WHERE username = $1"#,
        username
    )
    .fetch_optional(&state.db_connection_pool)
    .await?;

    match row {
        Some(r) => Ok(User {
            username: r.username,
            email: r.email,
            email_verified: r.email_verified,
            hashed_password: r.hashed_password,
            auth_level: r.auth_level,
            login_attempts: r.login_attempts,
            registration_ts: r.registration_ts,
            identity_provider: r.identity_provider,
        }),
        None => Err(anyhow!("User not found")),
    }
}

pub async fn update_google_user_email(
    state: Arc<AppState>,
    new_email: &str,
    email_verified: bool,
    sub: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        "UPDATE users SET email = $1, email_verified = $2 WHERE sub = $3",
        new_email,
        email_verified,
        sub
    )
    .execute(&state.db_connection_pool)
    .await?;

    Ok(())
}
