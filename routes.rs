use crate::route_handlers;
use axum::{routing::get, Router};

pub fn get_routes() -> Router {
    Router::new().route("/", get(route_handlers::hello_world))
}
