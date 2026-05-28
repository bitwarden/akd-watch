//! Wire-format DTOs for the akd-watch HTTP API.
//!
//! Single source of truth shared by the `akd_watch_web` server and any
//! client (notably `akd_watch_web_client`). Each DTO carries serde derives
//! for both directions, plus conversions to and from the underlying domain
//! types in this crate so server and client agree on the wire format
//! field-by-field.
//!
//! The client direction (`into_*`) returns [`WireError`] on malformed input
//! — the consumer maps that into whatever transport-layer error type it
//! prefers.

use chrono::{DateTime, TimeZone, Utc};
use ed25519_dalek::VerifyingKey as DalekVerifyingKey;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Ciphersuite, Epoch, EpochSignature, EpochSignatureV1, crypto::VerifyingKey};

/// Response body of `GET /info`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfiguration {
    pub keys: Vec<KeyInfo>,
    // Other configuration info
}

/// One verifying key as published on `/info`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyInfo {
    pub public_key: String,
    pub key_id: String,
    pub not_before: u64,
}

/// Response body of `GET /namespaces/:ns/audits/:epoch`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignatureResponse {
    pub version: u32,
    pub ciphersuite: Ciphersuite,
    pub namespace: String,
    pub timestamp: u64,
    pub epoch: Epoch,
    pub digest: String,
    pub signature: String,
    pub key_id: String,
}

/// Wire-format parsing error returned by the `into_*` conversions when a
/// DTO cannot be converted into its domain type. Carries a human-readable
/// reason for diagnostics; consumers wrap into their own transport error.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WireError(pub String);

// -- server direction: domain →z DTO --

impl From<&VerifyingKey> for KeyInfo {
    fn from(key: &VerifyingKey) -> Self {
        Self {
            public_key: hex::encode(key.verifying_key),
            key_id: key.key_id.to_string(),
            not_before: key.not_before.timestamp() as u64,
        }
    }
}

impl From<EpochSignature> for SignatureResponse {
    fn from(signature: EpochSignature) -> Self {
        let version = signature.version_int();
        match signature {
            EpochSignature::V1(sig) => SignatureResponse {
                version,
                ciphersuite: sig.ciphersuite,
                namespace: sig.namespace,
                timestamp: sig.timestamp as u64,
                epoch: sig.epoch,
                digest: hex::encode(sig.digest),
                signature: hex::encode(sig.signature),
                key_id: sig.key_id.to_string(),
            },
        }
    }
}

// -- client direction: DTO → domain (with validation) --

impl KeyInfo {
    pub fn into_verifying_key(self) -> Result<VerifyingKey, WireError> {
        let key_bytes = decode_hex_array::<32>(&self.public_key, "public_key")?;
        let verifying_key = DalekVerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| WireError(format!("invalid ed25519 public_key bytes: {e}")))?;
        let key_id = parse_uuid(&self.key_id, "key_id")?;
        let not_before = parse_unix_seconds(self.not_before)?;
        Ok(VerifyingKey {
            verifying_key,
            key_id,
            not_before,
        })
    }
}

impl SignatureResponse {
    pub fn into_epoch_signature(self) -> Result<EpochSignature, WireError> {
        let digest = decode_hex_vec(&self.digest, "digest")?;
        let signature = decode_hex_vec(&self.signature, "signature")?;
        let key_id = parse_uuid(&self.key_id, "key_id")?;

        let timestamp = i64::try_from(self.timestamp)
            .map_err(|_| WireError(format!("timestamp {} overflows i64", self.timestamp)))?;

        if !matches!(
            self.ciphersuite,
            Ciphersuite::ProtobufEd25519 | Ciphersuite::BincodeEd25519
        ) {
            return Err(WireError(format!(
                "unknown ciphersuite {:?}",
                self.ciphersuite
            )));
        }

        if self.version != 0x0001 {
            return Err(WireError(format!(
                "unsupported audit version {}",
                self.version
            )));
        }

        Ok(EpochSignature::V1(EpochSignatureV1 {
            ciphersuite: self.ciphersuite,
            namespace: self.namespace,
            timestamp,
            epoch: self.epoch,
            digest,
            signature,
            key_id,
        }))
    }
}

// -- parse helpers --

fn decode_hex_vec(value: &str, field: &'static str) -> Result<Vec<u8>, WireError> {
    hex::decode(value).map_err(|e| WireError(format!("could not hex-decode {field}: {e}")))
}

fn decode_hex_array<const N: usize>(
    value: &str,
    field: &'static str,
) -> Result<[u8; N], WireError> {
    let bytes = decode_hex_vec(value, field)?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| WireError(format!("{field} expected {N} bytes, got {}", v.len())))
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, WireError> {
    Uuid::parse_str(value).map_err(|e| WireError(format!("could not parse {field} as uuid: {e}")))
}

fn parse_unix_seconds(seconds: u64) -> Result<DateTime<Utc>, WireError> {
    let secs = i64::try_from(seconds)
        .map_err(|_| WireError(format!("not_before {seconds} overflows i64")))?;
    match Utc.timestamp_opt(secs, 0) {
        chrono::offset::LocalResult::Single(dt) => Ok(dt),
        _ => Err(WireError(format!(
            "not_before {seconds} is not a valid unix timestamp"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn well_formed_signature_response() -> SignatureResponse {
        SignatureResponse {
            version: 1,
            ciphersuite: Ciphersuite::ProtobufEd25519,
            namespace: "ns".into(),
            timestamp: 0,
            epoch: Epoch::new(0),
            digest: hex::encode([0u8; 32]),
            signature: hex::encode([0u8; 64]),
            key_id: "550e8400-e29b-41d4-a716-446655440000".into(),
        }
    }

    #[test]
    fn signature_response_unknown_ciphersuite_yields_wire_error() {
        let dto = SignatureResponse {
            ciphersuite: Ciphersuite::Unknown(0xDEAD),
            ..well_formed_signature_response()
        };
        let err = dto.into_epoch_signature().expect_err("should fail");
        assert!(err.0.contains("ciphersuite"), "reason: {}", err.0);
    }

    #[test]
    fn signature_response_unknown_version_yields_wire_error() {
        let dto = SignatureResponse {
            version: 99,
            ..well_formed_signature_response()
        };
        let err = dto.into_epoch_signature().expect_err("should fail");
        assert!(err.0.contains("version"), "reason: {}", err.0);
    }
}
