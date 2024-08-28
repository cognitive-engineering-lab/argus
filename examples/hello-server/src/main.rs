use axum::{routing::get, Router};
use tokio::net::TcpListener;

struct LoginAttempt {
  user_id: u64,
  password: String,
}

fn login(attempt: LoginAttempt) -> bool {
  todo!()
}

#[tokio::main]
async fn main() {
  let app = Router::new()
    .route("/login", get(login));

  let listener = TcpListener::bind("0.0.0.0:3000")
    .await.unwrap();
  axum::serve(listener, app).await.unwrap();
}
