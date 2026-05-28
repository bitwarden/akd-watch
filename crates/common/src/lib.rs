//! Server-side glue for akd-watch: storage backends, config loading, and
//! the `akd`-crate–tied configuration aliases. The wire-format types,
//! signing/verification primitives, and other protocol-level pieces live
//! in [`akd_watch_protocol`] and are re-exported from this crate so
//! existing consumers can keep using `akd_watch_common::*` for the moment;
//! new code should depend directly on `akd_watch_protocol` if it doesn't
//! need any of the server-side glue.

pub mod akd_configurations;
pub mod akd_storage_factory;
mod audit_blob_name;
pub mod config;
pub mod storage;

pub use akd_configurations::BitwardenV1Configuration;
pub use audit_blob_name::SerializableAuditBlobName;

#[cfg(test)]
pub use akd_configurations::TestAkdConfiguration;

// Export testing utilities when cfg(test) is enabled
#[cfg(any(test, feature = "testing"))]
pub mod testing;

// Re-export protocol-level items so existing `akd_watch_common::*` paths
// continue to work. New code should import directly from
// `akd_watch_protocol`.
pub use akd_watch_protocol::{
    BINCODE_CONFIG, Ciphersuite, Epoch, EpochSignature, EpochSignatureV1, NamespaceInfo,
    NamespaceStatus, SerializationError, SignError, VerifyError, crypto, tic_toc, timed_event,
    web_api,
};

use storage::signing_keys::{VerifyingKeyRepository, VerifyingKeyRepositoryError};
use uuid::Uuid;

/// Verify an `EpochSignature` by looking up its signing key in a
/// `VerifyingKeyRepository`. This is the server-side counterpart to
/// [`EpochSignature::verify_with_key`] — moved here (out of
/// `akd_watch_protocol`) because it depends on the storage trait, which
/// only server-side code knows about.
pub async fn verify_epoch_signature<R: VerifyingKeyRepository>(
    signature: &EpochSignature,
    verifying_key_repo: &R,
) -> Result<(), VerifyEpochSignatureError> {
    let signing_key_id = signature.signing_key_id();
    let verifying_key = verifying_key_repo
        .get_verifying_key(signing_key_id)
        .await
        .map_err(VerifyEpochSignatureError::Repository)?
        .ok_or(VerifyEpochSignatureError::VerifyingKeyNotFound(
            signing_key_id,
        ))?;
    signature
        .verify_with_key(&verifying_key)
        .map_err(VerifyEpochSignatureError::Verify)
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyEpochSignatureError {
    #[error("Verifying key not found with key id: {0}")]
    VerifyingKeyNotFound(Uuid),
    #[error("Verifying key repository error: {0}")]
    Repository(#[from] VerifyingKeyRepositoryError),
    #[error("Signature verify error: {0}")]
    Verify(#[from] VerifyError),
}
