#[derive(Clone)]
struct A;

async fn test() {
  crate::use_as_handler!(A);
}
