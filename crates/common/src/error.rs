use thiserror::Error;

#[derive(Debug, Error)]
pub enum AkdWatchError {
    #[error("Failed to parse epoch: {0}")]
    EpochParseError(#[from] std::num::ParseIntError),
    #[error("Local auditor error: {0:?}")]
    LocalAuditorError(akd::local_auditing::LocalAuditorError),
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Failed to parse epoch root hash: {0:?}")]
    EpochRootHashParseError(Vec<u8>),
}

impl From<akd::local_auditing::LocalAuditorError> for AkdWatchError {
    fn from(err: akd::local_auditing::LocalAuditorError) -> Self {
        AkdWatchError::LocalAuditorError(err)
    }
}
