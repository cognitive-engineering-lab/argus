use axum::{body::Body, extract::Extension, http::Request};
use super::*;

async fn handler(_: Request<Body>, _: Extension<String>) {}

fn test() {
    use_as_handler(handler);
}
