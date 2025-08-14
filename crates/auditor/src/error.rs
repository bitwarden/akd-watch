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

use std::array::TryFromSliceError;

use akd::errors::AkdError;

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Signature not found for epoch {0}")]
    SignatureNotFound(akd_watch_common::Epoch),
    #[error("Storage error: {0}")]
    StorageError(#[from] akd_watch_common::storage::AkdProofDirectoryError),
    #[error("Signing key error: {0}")]
    SigningKeyError(#[from] akd_watch_common::storage::signing_keys::SigningKeyRepositoryError),
    #[error("Verifying key error: {0}")]
    VerifyingKeyError(#[from] akd_watch_common::storage::signing_keys::VerifyingKeyRepositoryError),
    #[error("{0}")]
    SignatureStorageError(#[from] akd_watch_common::storage::signatures::SignatureRepositoryError),
    #[error("{0}")]
    VerifyError(#[from] akd_watch_common::VerifyError),
    #[error("Local Auditor error: {0:?}")]
    LocalAuditorError(akd::local_auditing::LocalAuditorError),
    #[error("Failed parsing blob hash: {0}")]
    BlobHashParseError(#[from] TryFromSliceError),
    #[error("Akd verification error: {0}")]
    AkdVerificationError(#[from] AkdError),
    #[error("Signing error: {0}")]
    SignError(#[from] akd_watch_common::SignError),
    #[error("Namespace repository error: {0}")]
    NamespaceRepositoryError(
        #[from] akd_watch_common::storage::namespaces::NamespaceRepositoryError,
    ),
}
