mod argument_not_extractor;
mod extract_self_mut;
mod extract_self_ref;
mod generics;
mod invalid_attrs;
mod missing_deserialize;
mod multiple_body_extractors;
mod multiple_paths;
mod not_a_function;
mod not_async;
mod not_send;
mod request_not_last;
mod too_many_extractors;
mod wrong_return_type;

#[macro_export]
macro_rules! use_as_handler {
  ($handler:expr) => {
    #[allow(unreachable_code)]
    {
      use axum::{routing::get, Router};
      let _app = Router::new().route("/", get($handler));
      axum::serve(todo!(), _app).await.unwrap();
    }
  };
}
