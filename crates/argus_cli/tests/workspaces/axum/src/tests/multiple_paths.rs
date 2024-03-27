use axum::extract::Path;

// NOTE: the original handler signature:
// async fn handler(_: Path<String>, _: Path<String>) {}
// is *now* a valid handler, whereas it wasn't in previous versions.
async fn handler(_: Path<String>, _: Path<String>) {}

async fn test() {
  crate::use_as_handler!(handler);
}
