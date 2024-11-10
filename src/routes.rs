use crate::{route_handlers, AppState};
use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

pub fn get_protected_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(route_handlers::hello_world))
        .route("/account/verifyEmail", post(route_handlers::verify_email))
        .route(
            "/account/changePassword",
            patch(route_handlers::change_password),
        )
}

pub fn get_open_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/account/register", post(route_handlers::register))
        .route("/account/login", post(route_handlers::login))
        .route(
            "/account/resetPassword",
            post(route_handlers::reset_password_request),
        )
        .route(
            "/account/resetPassword",
            patch(route_handlers::reset_password_response),
        )
}
