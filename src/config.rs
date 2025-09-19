use lettre::{
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        PoolConfig,
    },
    SmtpTransport,
};
use serde::Deserialize;
use std::str::FromStr;
use std::{fs::File, io::prelude::*};

use sqlx::{
    prelude::FromRow,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};

#[derive(Clone)]
pub struct AppState {
    pub db_connection_pool: Pool<Sqlite>,
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
    pub file: String,
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
    pub max_unsuccessful_login_attempts: i64,
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

    pub async fn get_db_pool(&self) -> Pool<Sqlite> {
        let connection_options = SqliteConnectOptions::from_str(self.database.file.as_str())
            .expect("Unable to open or create database")
            .create_if_missing(true);
        match SqlitePoolOptions::new()
            .max_connections(self.database.pool_size)
            .connect_with(connection_options)
            .await
        {
            Ok(val) => val,
            Err(e) => panic!("Unable to create connection pool due to {}", e),
        }
    }
}
