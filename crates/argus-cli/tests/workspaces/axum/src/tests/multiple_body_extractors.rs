use axum::body::Bytes;

async fn handler(_: String, _: Bytes) {}

async fn test() {
  crate::use_as_handler!(handler);
}
