use std::net::SocketAddr;
use axum::{Router, routing::post, Json};

async fn fake_main() {
    let app = Router::new()
        .route("/test", post(test));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum_server::Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// #[derive(serde::Deserialize)] Without this the error is caused
struct Test {}

async fn test(
    Json(_): Json<Test>
) {}
