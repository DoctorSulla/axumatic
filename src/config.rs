use lettre::{
    SmtpTransport,
    transport::smtp::{
        PoolConfig,
        authentication::{Credentials, Mechanism},
    },
};
use serde::Deserialize;
use std::env;
use std::{fs::File, io::prelude::*};
use std::{str::FromStr, time::Duration};
use tracing::{Level, event};

use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

#[derive(Clone)]
pub struct AppState {
    pub db_connection_pool: Pool<Postgres>,
    pub email_connection_pool: SmtpTransport,
    pub config: Config,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub email: SmtpConfig,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub pool_size: u32,
    pub username: String,
    pub password: Option<String>,
    pub connection_url: String,
}

impl DatabaseConfig {
    pub fn get_connection_string(&self) -> String {
        let password = self.password.clone().unwrap();
        format!(
            "postgresql://{}:{}@{}",
            self.username, password, self.connection_url
        )
    }
}

impl SmtpConfig {
    pub fn get_password(&self) -> String {
        self.password.clone().unwrap()
    }
}

#[derive(Deserialize, Clone)]
pub struct SmtpConfig {
    pub server_url: String,
    pub username: String,
    pub password: Option<String>,
    pub pool_size: u32,
    pub send_emails: bool,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub request_timeout: u64,
    pub max_unsuccessful_login_attempts: i32,
    pub session_length_in_days: i64,
    pub google_client_id: String,
}

#[derive(Deserialize, Clone)]
pub enum AuthLevel {
    User,
    Admin,
}

impl TryFrom<String> for AuthLevel {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "user" => Ok(AuthLevel::User),
            "admin" => Ok(AuthLevel::Admin),
            _ => Err("Invalid auth level".to_string()),
        }
    }
}

impl From<AuthLevel> for String {
    fn from(value: AuthLevel) -> Self {
        match value {
            AuthLevel::User => "user".to_string(),
            AuthLevel::Admin => "admin".to_string(),
        }
    }
}

pub fn get_config() -> Config {
    let environment = env::var("AXUMATIC_ENVIRONMENT")
        .unwrap_or_else(|_| "TEST".to_string());

    // Open and parse the config file
    let mut file = match environment.as_str() {
        "PROD" => File::open("./config.toml").expect("Couldn't open config file"),
        _ => File::open("./test-config.toml").expect("Couldn't open config file"),
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Couldn't read config");

    let mut config: Config = toml::from_str(contents.as_str()).expect("Couldn't parse config");
    config.populate_passwords();
    config
}

impl Config {
    pub fn get_email_pool(&self) -> SmtpTransport {
        SmtpTransport::starttls_relay(self.email.server_url.as_str())
            .expect("Unable to create email connection pool")
            // Add credentials for authentication
            .credentials(Credentials::new(
                self.email.username.to_owned(),
                self.email.get_password(),
            ))
            // Configure expected authentication mechanism
            .authentication(vec![Mechanism::Plain])
            // Connection pool settings
            .pool_config(PoolConfig::new().max_size(self.email.pool_size))
            .build()
    }

    pub async fn get_db_pool(&self) -> Pool<Postgres> {
        let connection_string = &self.database.get_connection_string();

        event!(
            Level::INFO,
            "Attempting to connect to Postgres with {}@{}",
            &self.database.username,
            &self.database.connection_url
        );

        let connection_options =
            sqlx::postgres::PgConnectOptions::from_str(connection_string).unwrap();

        PgPoolOptions::new()
            .max_connections(self.database.pool_size)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(60))
            .connect_lazy_with(connection_options)
    }

    pub fn populate_passwords(&mut self) {
        let pg_password = env::var("AXUMATIC_PG_PASSWORD")
            .expect("AXUMATIC_PG_PASSWORD variable not set");
        let smtp_password = env::var("AXUMATIC_SMTP_PASSWORD")
            .expect("AXUMATIC_SMTP_PASSWORD variable not set");

        self.database.password = Some(pg_password);
        self.email.password = Some(smtp_password);
    }
}
