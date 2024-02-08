use axum::extract::Path;
use super::*;

async fn handler(_: Path<String>, _: Path<String>) {}

fn test() {
    use_as_handler(handler);
}
