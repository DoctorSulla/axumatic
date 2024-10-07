use crate::route_handlers;
use axum::{
    routing::{get, post},
    Router,
};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(route_handlers::hello_world))
        .route("/account/register", post(route_handlers::register))
        .route("/account/login", post(route_handlers::login))
        .route("/account/verifyEmail", post(route_handlers::verify_email))
        .route(
            "/account/resetPassword",
            post(route_handlers::reset_password),
        )
        .route(
            "/account/changePassword",
            get(route_handlers::change_password),
        )
}
