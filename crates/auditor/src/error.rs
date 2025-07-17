// use thiserror::Error;

// use akd_watch_common::{AkdWatchError, Epoch};

// #[derive(Debug, Error)]
// pub enum AuditorError {
//     #[error("{0}")]
//     AkdWatchError(#[from] AkdWatchError),
//     #[error("Failed to download: {0}")]
//     DownloadError(#[from] reqwest::Error),
//     #[error("Unable to find signature for epoch {0}")]
//     SignatureNotFound(Epoch),
// }
