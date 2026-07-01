use akd_watch_protocol::{Epoch, EpochSignature, crypto::VerifyingKey};

use crate::error::VerifyAuditError;

/// Verify the ed25519 signature on a parsed audit against a resolved key.
///
/// Any verification failure — signature mismatch, length error, or
/// serialization error — collapses to `AuditorSignatureInvalid`; the caller's
/// response is identical in all of them: stop trusting the auditor. The
/// underlying `VerifyError` variant is logged at WARN before collapsing so
/// operators can still see exactly which sub-case fired.
pub(crate) fn verify_signature(
    signature: &EpochSignature,
    key: &VerifyingKey,
    namespace: &str,
    epoch: Epoch,
) -> Result<(), VerifyAuditError> {
    signature.verify_with_key(key).map_err(|e| {
        tracing::warn!(
            namespace,
            epoch = %epoch,
            key_id = %key.key_id,
            error = %e,
            "audit signature failed cryptographic verification — auditor produced bad bytes"
        );
        VerifyAuditError::AuditorSignatureInvalid {
            namespace: namespace.to_string(),
            epoch,
            key_id: key.key_id,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use akd_watch_protocol::NamespaceInfo;
    use chrono::{Duration, Utc};
    use ed25519_dalek::SigningKey as DalekSigningKey;
    use rand::RngCore;
    use uuid::Uuid;

    fn fixture(namespace: &str, epoch: Epoch) -> (EpochSignature, VerifyingKey) {
        let mut secret = [0u8; 32];
        rand::rng().fill_bytes(&mut secret);
        let signing = DalekSigningKey::from_bytes(&secret);
        let verifying = signing.verifying_key();
        let key_id = Uuid::new_v4();
        let now = Utc::now();
        let common_signing = akd_watch_protocol::crypto::SigningKey::new(
            signing,
            key_id,
            now,
            now + Duration::days(30),
        );
        let ns = NamespaceInfo {
            configuration:
                akd_watch_protocol::akd_configurations::AkdConfiguration::WhatsAppV1Configuration,
            name: namespace.to_string(),
            log_directory: "log".to_string(),
            last_verified_epoch: None,
            starting_epoch: Epoch::new(0),
            status: akd_watch_protocol::NamespaceStatus::Online,
        };
        let mut root_hash = [0u8; 32];
        rand::rng().fill_bytes(&mut root_hash);
        let sig = EpochSignature::sign(ns, epoch, root_hash, &common_signing).expect("sign");
        let vk = VerifyingKey {
            verifying_key: verifying,
            key_id,
            not_before: now,
        };
        (sig, vk)
    }

    #[test]
    fn verify_signature_succeeds_for_valid_signature() {
        let epoch = Epoch::new(7);
        let (sig, key) = fixture("ns", epoch);
        verify_signature(&sig, &key, "ns", epoch).expect("valid signature");
    }

    #[test]
    fn verify_signature_returns_auditor_failure_when_signature_corrupted() {
        let epoch = Epoch::new(7);
        let (sig, key) = fixture("ns", epoch);
        let tampered = match sig {
            EpochSignature::V1(mut v1) => {
                v1.signature[0] ^= 0xFF;
                EpochSignature::V1(v1)
            }
        };
        let err = verify_signature(&tampered, &key, "ns", epoch).expect_err("should fail");
        match err {
            VerifyAuditError::AuditorSignatureInvalid {
                namespace,
                epoch: e,
                key_id,
            } => {
                assert_eq!(namespace, "ns");
                assert_eq!(e, epoch);
                assert_eq!(key_id, key.key_id);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
