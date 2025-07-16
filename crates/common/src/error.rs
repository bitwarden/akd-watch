use thiserror::Error;

#[derive(Debug, Error)]
pub enum AkdWatchError {
    #[error("Failed to parse epoch: {0}")]
    EpochParseError(#[from] std::num::ParseIntError),
}
