async fn handler() {
  let _rc = std::rc::Rc::new(());
  // let _arc = std::sync::Arc::new(());
  async {}.await;
}

async fn test() {
  crate::use_as_handler!(handler);
}
