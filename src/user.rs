use anyhow::anyhow;

use crate::{config::AppState, default_route_handlers::User};
use std::sync::Arc;

pub async fn get_user_by_email(state: Arc<AppState>, email: &str) -> Result<User, anyhow::Error> {
    let user = sqlx::query_as::<_, User>("select * from users where username=$1")
        .bind(email)
        .fetch_optional(&state.db_connection_pool)
        .await?;

    match user {
        Some(user) => Ok(user),
        None => Err(anyhow!("User not found")),
    }
}
