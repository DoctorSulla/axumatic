use serde::Deserialize;

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
}
