use akd_watch_protocol::Epoch;
use uuid::Uuid;

/// Common HTTP-layer errors. Reused as a wrapped variant inside the
/// method-specific error types so HTTP failures share one shape.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// akd-watch is currently unavailable. Covers both network-level
    /// failures (DNS, TLS, connection refused, body stream broken) and
    /// 5xx responses from the server. Retry with backoff; if persistent,
    /// check connectivity or escalate to whoever runs the akd-watch. The
    /// `reason` carries the underlying transport error or status+body.
    #[error("akd-watch unavailable: {reason}")]
    Unavailable { reason: String },

    /// Either the server rejected our request with a 4xx, or we could not
    /// interpret the response body. Both indicate a wire-format mismatch
    /// between this client and the akd-watch — align versions or file a
    /// schema bug. The `reason` carries the status code (for the 4xx case)
    /// or the parse-failure detail (for the body case).
    #[error("protocol mismatch with akd-watch: {reason}")]
    ProtocolMismatch { reason: String },
}

/// Errors returned by [`crate::ClientBuilder::build`].
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("namespace must not be empty")]
    EmptyNamespace,

    #[error("invalid base URL: {reason}")]
    InvalidBaseUrl { reason: String },

    #[error(transparent)]
    Http(#[from] HttpError),

    #[error("namespace '{namespace}' not found at this akd-watch")]
    NamespaceNotFound { namespace: String },

    #[error("pinned verifying key {key_id} is not in the akd-watch's published key set")]
    PinnedKeyMissing { key_id: Uuid },
}

/// Errors returned by [`crate::Client::verify_audit`].
#[derive(Debug, thiserror::Error)]
pub enum VerifyAuditError {
    #[error(transparent)]
    Http(#[from] HttpError),

    #[error("no audit signature available for namespace '{namespace}' epoch {epoch}")]
    AuditNotAvailable { namespace: String, epoch: Epoch },

    #[error(
        "auditor failure: signature for namespace '{namespace}' epoch {epoch} did not validate (key {key_id})"
    )]
    AuditorSignatureInvalid {
        namespace: String,
        epoch: Epoch,
        key_id: Uuid,
    },

    #[error(
        "root hash mismatch for namespace '{namespace}' epoch {epoch}: expected {expected_hex}, got {actual_hex}"
    )]
    RootHashMismatch {
        namespace: String,
        epoch: Epoch,
        expected_hex: String,
        actual_hex: String,
    },
}
