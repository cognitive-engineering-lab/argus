use axum::{async_trait, extract::FromRequestParts, http::request::Parts};

struct A;

#[async_trait]
impl<S> FromRequestParts<S> for A
where
  S: Send + Sync,
{
  type Rejection = ();

  async fn from_request_parts(
    _req: &mut Parts,
    _state: &S,
  ) -> Result<Self, Self::Rejection> {
    unimplemented!()
  }
}

impl A {
  async fn handler(&self) {}
}

async fn test() {
  crate::use_as_handler!(A::handler);
}
