use akd_watch_protocol::crypto::VerifyingKey;
use uuid::Uuid;

use crate::error::BuildError;

/// Validate that every pinned key is in the server's published key set.
/// Each pinned key must match a server key by both `key_id` and public key
/// bytes. The first mismatch produces `PinnedKeyMissing`
pub(crate) fn validate_pinned_against_server(
    pinned: &[VerifyingKey],
    server: &[VerifyingKey],
) -> Result<(), BuildError> {
    for pin in pinned {
        match server.iter().find(|s| s.key_id == pin.key_id) {
            Some(s) if s.verifying_key == pin.verifying_key => continue,
            Some(_) => {
                tracing::warn!(
                    key_id = %pin.key_id,
                    "pinned key id is present in /info but the public key bytes do not match — possible MITM or auditor compromise"
                );
                return Err(BuildError::PinnedKeyMissing { key_id: pin.key_id });
            }
            None => {
                tracing::warn!(
                    key_id = %pin.key_id,
                    "pinned key id is not present in /info — auditor may have rotated and re-audited, discarding the old key"
                );
                return Err(BuildError::PinnedKeyMissing { key_id: pin.key_id });
            }
        }
    }
    Ok(())
}

/// Look up `audit_key_id` in the cached server-published key set.
pub(crate) fn find_in_server(audit_key_id: Uuid, server: &[VerifyingKey]) -> Option<VerifyingKey> {
    server.iter().find(|k| k.key_id == audit_key_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    fn make_key() -> VerifyingKey {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let signing = SigningKey::from_bytes(&bytes);
        VerifyingKey {
            verifying_key: signing.verifying_key(),
            key_id: Uuid::new_v4(),
            not_before: Utc::now(),
        }
    }

    #[test]
    fn validate_passes_when_all_pinned_keys_are_on_server() {
        let a = make_key();
        let b = make_key();
        // Server has both A and B; caller pinned only A.
        validate_pinned_against_server(&[a.clone()], &[a, b]).expect("should pass");
    }

    #[test]
    fn validate_fails_when_pinned_key_id_missing_from_server() {
        let pinned = make_key();
        let server_only = make_key();
        let err = validate_pinned_against_server(&[pinned.clone()], &[server_only])
            .expect_err("should be PinnedKeyMissing");
        match err {
            BuildError::PinnedKeyMissing { key_id } => assert_eq!(key_id, pinned.key_id),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_fails_when_key_id_matches_but_pubkey_does_not() {
        let pinned = make_key();
        let mut imposter = make_key();
        imposter.key_id = pinned.key_id;
        let err = validate_pinned_against_server(&[pinned.clone()], &[imposter])
            .expect_err("should be PinnedKeyMissing");
        match err {
            BuildError::PinnedKeyMissing { key_id } => assert_eq!(key_id, pinned.key_id),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
