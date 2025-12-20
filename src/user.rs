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
    sqlx::query_as::<_, User>("select * from users where email=$1")
        .bind(email)
        .fetch_optional(&state.db_connection_pool)
        .await?
        .ok_or_else(|| anyhow!("User not found"))
}

pub async fn get_user_by_sub(state: Arc<AppState>, sub: &str) -> Result<User, anyhow::Error> {
    sqlx::query_as::<_, User>("select * from users where sub=$1")
        .bind(sub)
        .fetch_optional(&state.db_connection_pool)
        .await?
        .ok_or_else(|| anyhow!("User not found"))
}

pub async fn get_user_by_username(
    state: Arc<AppState>,
    username: &str,
) -> Result<User, anyhow::Error> {
    sqlx::query_as::<_, User>("select * from users where username=$1")
        .bind(username)
        .fetch_optional(&state.db_connection_pool)
        .await?
        .ok_or_else(|| anyhow!("User not found"))
}

pub async fn update_google_user_email(
    state: Arc<AppState>,
    new_email: &str,
    email_verified: bool,
    sub: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query("update users set email=$1, email_verified=$2 where sub=$3")
        .bind(new_email)
        .bind(email_verified)
        .bind(sub)
        .execute(&state.db_connection_pool)
        .await?;

    Ok(())
}
