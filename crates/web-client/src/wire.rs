use akd_watch_protocol::web_api::WireError;

use crate::error::HttpError;

/// Map a parse failure from `akd_watch_protocol::web_api` into the client's
/// transport-layer error type. Both directions of mismatch (server returned
/// a value we cannot interpret) collapse to `ProtocolMismatch` with the
/// reason preserved for diagnostics.
impl From<WireError> for HttpError {
    fn from(e: WireError) -> Self {
        HttpError::ProtocolMismatch { reason: e.0 }
    }
}
