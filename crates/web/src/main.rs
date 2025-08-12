use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber;

mod error;
mod routes;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Build API
    let app = Router::new().merge(routes::api_routes());

    // Start server
    // TODO: Make address configurable
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://{}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
