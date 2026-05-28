use std::array::TryFromSliceError;

use bincode::{Decode, Encode};
use ed25519_dalek::ed25519::signature::SignerMut;
use ed25519_dalek::{SignatureError, Verifier};
use prost::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Ciphersuite, Epoch, NamespaceInfo,
    crypto::{SigningKey, VerifyingKey},
    error::SerializationError,
};

#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
#[serde(tag = "audit_version")]
pub enum EpochSignature {
    V1(EpochSignatureV1),
}

impl EpochSignature {
    pub fn version_int(&self) -> u32 {
        match self {
            EpochSignature::V1(_) => 0x00_01,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct EpochSignatureV1 {
    pub ciphersuite: Ciphersuite,
    pub namespace: String,
    pub timestamp: i64,
    pub epoch: Epoch,
    pub digest: Vec<u8>,
    pub signature: Vec<u8>,
    #[bincode(with_serde)]
    pub key_id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("Signature verification failed")]
    SignatureVerificationFailed(#[from] SignatureError),
    #[error("Signature length error: expected {expected}, got {actual}")]
    SignatureLengthError { expected: usize, actual: usize },
    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializationError),
    #[error("Verifying key not found with key id: {0}")]
    VerifyingKeyNotFound(Uuid),
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializationError),
}

impl EpochSignatureV1 {
    pub(crate) fn verify(&self, verifying_key: &VerifyingKey) -> Result<(), VerifyError> {
        let message = self.to_message().to_vec()?;

        let signature =
            ed25519_dalek::Signature::from_bytes(self.signature.as_slice().try_into().map_err(
                |_| VerifyError::SignatureLengthError {
                    expected: 64,
                    actual: self.signature.len(),
                },
            )?);

        verifying_key
            .verifying_key
            .verify(&message.to_vec(), &signature)
            .map_err(VerifyError::from)
    }

    fn to_message(&self) -> EpochSignedMessage {
        EpochSignedMessage {
            ciphersuite: self.ciphersuite,
            namespace: self.namespace.clone(),
            timestamp: self.timestamp,
            epoch: self.epoch,
            digest: self.digest.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Encode)]
pub struct EpochSignedMessage {
    pub(crate) ciphersuite: Ciphersuite,
    pub(crate) namespace: String,
    pub(crate) timestamp: i64,
    pub(crate) epoch: Epoch,
    pub(crate) digest: Vec<u8>,
}

impl EpochSignedMessage {
    pub fn to_vec(&self) -> Result<Vec<u8>, SerializationError> {
        match self.ciphersuite {
            Ciphersuite::ProtobufEd25519 => {
                Ok(crate::proto::types::SignatureMessage::from(self).encode_to_vec())
            }
            Ciphersuite::BincodeEd25519 => Ok(bincode::encode_to_vec(self, crate::BINCODE_CONFIG)?),
            _ => Err(SerializationError::UnknownFormat(format!(
                "{:?}",
                self.ciphersuite
            ))),
        }
    }
}

impl EpochSignature {
    pub fn sign(
        namespace: NamespaceInfo,
        epoch: Epoch,
        epoch_root_hash: [u8; 32],
        signing_key: &SigningKey,
    ) -> Result<Self, SignError> {
        let message = EpochSignedMessage {
            ciphersuite: Ciphersuite::default(),
            namespace: namespace.name.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            epoch,
            digest: epoch_root_hash.to_vec(),
        };
        let signature = signing_key
            .signing_key()
            .write()
            .expect("Poisoned signing key")
            .sign(&message.to_vec()?);
        Ok(EpochSignature::V1(EpochSignatureV1 {
            ciphersuite: message.ciphersuite,
            namespace: message.namespace,
            timestamp: message.timestamp,
            epoch: message.epoch,
            digest: message.digest,
            signature: signature.to_bytes().to_vec(),
            key_id: signing_key.key_id(),
        }))
    }

    pub fn digest(&self) -> Vec<u8> {
        match self {
            EpochSignature::V1(signature) => signature.digest.clone(),
        }
    }

    pub fn digest_hex(&self) -> String {
        hex::encode(self.digest())
    }

    pub fn epoch_root_hash(&self) -> Result<[u8; 32], TryFromSliceError> {
        match self {
            EpochSignature::V1(signature) => signature.digest.as_slice().try_into(),
        }
    }

    pub fn signing_key_id(&self) -> Uuid {
        match self {
            EpochSignature::V1(signature) => signature.key_id,
        }
    }

    /// Verify the signature against a specific verifying key. Use this when
    /// the caller has already resolved the trusted key (e.g. an HTTP client
    /// that fetched the publisher's key set). For the repository-based
    /// lookup variant, see `akd_watch_common::verify_epoch_signature`.
    pub fn verify_with_key(&self, verifying_key: &VerifyingKey) -> Result<(), VerifyError> {
        match self {
            EpochSignature::V1(signature) => signature.verify(verifying_key),
        }
    }
}
