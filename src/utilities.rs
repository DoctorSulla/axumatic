use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use lettre::{Message, Transport};
use rand::{Rng, thread_rng};

use tracing::{Level, event};

use std::sync::Arc;

use crate::AppState;

use chrono::Utc;

#[derive(Debug)]
pub struct Email<'a> {
    pub from: &'a str,
    pub reply_to: Option<&'a str>,
    pub to: &'a str,
    pub subject: String,
    pub body: String,
}
pub async fn send_email(state: Arc<AppState>, email: Email<'_>) -> Result<(), anyhow::Error> {
    event!(Level::INFO, "The email to be sent to the user is {:?}", {
        &email
    });

    if state.config.email.send_emails {
        let email = Message::builder()
            .from(email.from.parse()?)
            .reply_to(email.reply_to.unwrap_or_default().parse()?)
            .to(email.to.parse()?)
            .subject(email.subject)
            .body(email.body)?;
        //Send the email via remote relay
        let _ = state.email_connection_pool.send(&email);
    }
    Ok(())
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Unable to hash password")
        .to_string()
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(hash).expect("Unable to parse hash");
    argon2.verify_password(password.as_bytes(), &password_hash).is_ok()
}

pub fn generate_unique_id(length: u8) -> String {
    let mut rng = thread_rng();
    const CHARACTER_SET: [char; 36] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    ];
    (0..length)
        .map(|_| CHARACTER_SET[rng.gen_range(0..CHARACTER_SET.len())])
        .collect()
}

pub async fn start_session_cleaner(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let now = Utc::now().timestamp();
            let delete = sqlx::query("DELETE FROM sessions WHERE $1 > expiry")
                .bind(now)
                .execute(&state.db_connection_pool)
                .await;
            match delete {
                Ok(_v) => event!(Level::INFO, "Expired sessions deleted"),
                Err(e) => event!(Level::WARN, "Failed to delete sessions due to {}", e),
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });
}
