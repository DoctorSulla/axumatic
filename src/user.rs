use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::config::AppState;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct User {
    pub username: String,
    pub email: String,
    pub hashed_password: String,
    pub auth_level: String,
    pub login_attempts: i32,
    pub registration_ts: i64,
    pub identity_provider: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Profile {
    pub username: String,
    pub email: String,
    pub auth_level: String,
    pub identity_provider: String,
    pub registration_ts: i64,
}

impl From<User> for Profile {
    fn from(value: User) -> Self {
        Self {
            username: value.username,
            email: value.email,
            auth_level: value.auth_level,
            identity_provider: value.identity_provider,
            registration_ts: value.registration_ts,
        }
    }
}

pub async fn get_user_by_email(state: Arc<AppState>, email: &str) -> Result<User, anyhow::Error> {
    let user = sqlx::query_as::<_, User>("select * from users where email=$1")
        .bind(email)
        .fetch_optional(&state.db_connection_pool)
        .await?;

    match user {
        Some(user) => Ok(user),
        None => Err(anyhow!("User not found")),
    }
}
