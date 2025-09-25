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
}

#[derive(Deserialize, Clone)]
pub struct SmtpConfig {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub pool_size: u32,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub request_timeout: u64,
    pub max_unsuccessful_login_attempts: i32,
}

#[derive(Deserialize, Clone)]
pub enum AuthLevel {
    Unverified,
    Verified,
    Admin,
}

impl TryFrom<String> for AuthLevel {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "unverified" => Ok(AuthLevel::Unverified),
            "verified" => Ok(AuthLevel::Verified),
            "admin" => Ok(AuthLevel::Admin),
            _ => Err("Invalid auth level".to_string()),
        }
    }
}

impl From<AuthLevel> for String {
    fn from(value: AuthLevel) -> Self {
        match value {
            AuthLevel::Unverified => "admin".to_string(),
            AuthLevel::Admin => "unverified".to_string(),
            AuthLevel::Verified => "verified".to_string(),
        }
    }
}

pub fn get_config() -> Config {
    // Open and parse the config file
    let mut file = File::open("./config.toml").expect("Couldn't open config file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Couldn't convert config file to string");

    toml::from_str(contents.as_str()).expect("Couldn't parse config")
}

impl Config {
    pub fn get_email_pool(&self) -> SmtpTransport {
        SmtpTransport::starttls_relay(self.email.server_url.as_str())
            .expect("Unable to create email connection pool")
            // Add credentials for authentication
            .credentials(Credentials::new(
                self.email.username.to_owned(),
                self.email.password.to_owned(),
            ))
            // Configure expected authentication mechanism
            .authentication(vec![Mechanism::Plain])
            // Connection pool settings
            .pool_config(PoolConfig::new().max_size(self.email.pool_size))
            .build()
    }

    pub async fn get_db_pool(&self) -> Pool<Postgres> {
        let mut vars = env::vars();
        let (_key, password) = vars
            .find(|kv| kv.0 == "PG_PASSWORD")
            .expect("PG_PASSWORD variable not set");
        let connection_string = format!(
            "postgresql://neondb_owner:{password}@ep-summer-wind-abumg0f3-pooler.eu-west-2.aws.neon.tech/neondb?sslmode=require"
        );

        let connection_options =
            sqlx::postgres::PgConnectOptions::from_str(&connection_string).unwrap();

        PgPoolOptions::new()
            .max_connections(self.database.pool_size)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(60))
            .connect_lazy_with(connection_options)
    }
}
