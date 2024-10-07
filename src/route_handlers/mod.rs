use axum::{http::StatusCode, response::Html};

pub async fn hello_world() -> Result<Html<String>, StatusCode> {
    Ok(Html("Hello World".to_string()))
}

pub async fn register() -> Result<Html<String>, StatusCode> {
    Ok(Html("Register".to_string()))
}

pub async fn login() -> Result<Html<String>, StatusCode> {
    Ok(Html("Register".to_string()))
}

pub async fn verify_email() -> Result<Html<String>, StatusCode> {
    Ok(Html("Register".to_string()))
}

pub async fn change_password() -> Result<Html<String>, StatusCode> {
    Ok(Html("Register".to_string()))
}

pub async fn reset_password() -> Result<Html<String>, StatusCode> {
    Ok(Html("Register".to_string()))
}
