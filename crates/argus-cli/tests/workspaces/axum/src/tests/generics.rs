async fn handler<T>() {}

async fn test() {
  crate::use_as_handler!(handler);
}
