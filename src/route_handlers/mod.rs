use axum::extract::Json;
use axum::{http::StatusCode, response::Html};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RegistrationDetails {
    username: String,
    email: String,
    password: String,
    confirm_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginDetails {
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChangePassword {
    password: String,
    confirm_password: String,
}

pub async fn hello_world() -> Result<Html<String>, StatusCode> {
    Ok(Html("Hello World".to_string()))
}

pub async fn register(
    Json(registration_details): Json<RegistrationDetails>,
) -> Result<Html<String>, StatusCode> {
    Ok(Html(registration_details.email))
}

pub async fn login() -> Result<Html<String>, StatusCode> {
    Ok(Html("Login".to_string()))
}

pub async fn verify_email() -> Result<Html<String>, StatusCode> {
    Ok(Html("Verify Email".to_string()))
}

pub async fn change_password() -> Result<Html<String>, StatusCode> {
    Ok(Html("Change Password".to_string()))
}

pub async fn reset_password() -> Result<Html<String>, StatusCode> {
    Ok(Html("Reset Password".to_string()))
}
