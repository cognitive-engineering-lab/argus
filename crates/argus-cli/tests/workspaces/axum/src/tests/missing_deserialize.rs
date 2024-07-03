use axum::Json;

// #[derive(serde::Deserialize)] // Without this the error is caused
struct Test {}

async fn handler(Json(_): Json<Test>) {}

async fn test() {
  crate::use_as_handler!(handler)
}
