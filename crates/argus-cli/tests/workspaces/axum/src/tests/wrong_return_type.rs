async fn handler() -> bool {
  false
}

async fn test() {
  crate::use_as_handler!(handler);
}
