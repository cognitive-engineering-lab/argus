async fn handler(foo: bool) {}

async fn test() {
  crate::use_as_handler!(handler);
}
