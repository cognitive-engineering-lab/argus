use axum::{body::Body, extract::Extension, http::Request};

async fn handler(_: Request<Body>, _: Extension<String>) {}

async fn test() {
  crate::use_as_handler!(handler);
}
