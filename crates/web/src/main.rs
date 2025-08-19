use akd_watch_web::start;
use tracing::error;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    if let Err(e) = start().await {
        error!(error = %e, "Application error");
        std::process::exit(1);
    }
}
