use axum::body::Bytes;
use super::*;

async fn handler(_: String, _: Bytes) {}

fn test() {
    use_as_handler(handler);
}
