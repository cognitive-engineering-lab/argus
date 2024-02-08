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

use axum::handler::Handler;

pub fn use_as_handler<H, T, S>(handler: H)
where
  H: Handler<T, S>,
  T: 'static,
  S: Clone + Send + Sync + 'static,
{
}
