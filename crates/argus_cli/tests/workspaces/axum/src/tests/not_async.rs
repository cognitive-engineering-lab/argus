fn handler() {}

async fn test() {
  crate::use_as_handler!(handler);
}
