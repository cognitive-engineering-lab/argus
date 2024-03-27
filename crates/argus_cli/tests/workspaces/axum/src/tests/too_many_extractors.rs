use axum::http::Uri;
async fn handler(
  _e1: Uri,
  _e2: Uri,
  _e3: Uri,
  _e4: Uri,
  _e5: Uri,
  _e6: Uri,
  _e7: Uri,
  _e8: Uri,
  _e9: Uri,
  _e10: Uri,
  _e11: Uri,
  _e12: Uri,
  _e13: Uri,
  _e14: Uri,
  _e15: Uri,
  _e16: Uri,
  _e17: Uri,
) {
}

async fn test() {
  crate::use_as_handler!(handler);
}
