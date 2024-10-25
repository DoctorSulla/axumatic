use anyhow::Error;
use sqlx::SqlitePool;

pub async fn create_tables(pool: SqlitePool) -> Result<(), Error> {
    let _users = sqlx::query(
        "CREATE TABLE IF NOT EXISTS users(
email VARCHAR(100) unique,
username VARCHAR(50) unique PRIMARY KEY,
hashed_password VARCHAR(100),
login_attempts INTEGER,
auth_level INTEGER DEFAULT 0
)",
    )
    .execute(&pool)
    .await?;

    let _codes = sqlx::query(
        "CREATE TABLE IF NOT EXISTS codes(
id INTEGER AUTO_INCREMENT PRIMARY KEY,
code_type VARCHAR(20),
email VARCHAR(100),
code VARCHAR(30),
created_ts VARCHAR(30),
expiry_ts VARCHAR(30),
used INTEGER default 0
)",
    )
    .execute(&pool)
    .await?;

    Ok(())
}
