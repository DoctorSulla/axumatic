use axum::{http::StatusCode, response::Html};

pub async fn hello_world() -> Result<Html<String>, StatusCode> {
    Ok(Html("Hello World".to_string()))
}
