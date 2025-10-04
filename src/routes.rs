use crate::{AppState, default_route_handlers};
use axum::{
    Router,
    routing::{get, patch, post},
};
use std::sync::Arc;

pub fn get_protected_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/account/verifyEmail",
            post(default_route_handlers::verify_email),
        )
        .route(
            "/account/changePassword",
            patch(default_route_handlers::change_password),
        )
        .route("/account/profile", get(default_route_handlers::get_profile))
        .route("/account/logout", get(default_route_handlers::logout))
        .route(
            "/account/verificationEmail",
            get(default_route_handlers::resend_verification_email),
        )
}

pub fn get_open_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/account/register", post(default_route_handlers::register))
        .route("/account/login", post(default_route_handlers::login))
        .route(
            "/account/login/google",
            post(default_route_handlers::google_login),
        )
        .route(
            "/account/resetPassword",
            post(default_route_handlers::password_reset_initiate),
        )
        .route(
            "/account/resetPassword",
            patch(default_route_handlers::password_reset_complete),
        )
        .route("/healthCheck", get(default_route_handlers::health_check))
        .route("/nonce", get(default_route_handlers::get_nonce))
}
