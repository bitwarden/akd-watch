use thiserror::Error;

use crate::Ciphersuite;

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
    #[error("Unsupported ciphersuite: {0:?}")]
    UnsupportedCiphersuite(Ciphersuite),
    #[error("Signature parse error: {0}")]
    SignatureParseError(#[from] ed25519_dalek::SignatureError),
    #[error("Signature length error: expected {expected}, got {actual}")]
    SignatureLengthError {
        expected: usize,
        actual: usize,
    },
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("poisoned signing key")]
    PoisonedSigningKey,
}

impl From<akd::local_auditing::LocalAuditorError> for AkdWatchError {
    fn from(err: akd::local_auditing::LocalAuditorError) -> Self {
        AkdWatchError::LocalAuditorError(err)
    }
}
