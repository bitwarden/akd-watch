use std::array::TryFromSliceError;

use ed25519_dalek::ed25519::signature::SignerMut;
use ed25519_dalek::{SignatureError, Verifier};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Ciphersuite, Epoch, NamespaceInfo,
    crypto::{SigningKey, VerifyingKey},
    error::SerializationError,
    storage::signing_keys::VerifyingKeyRepository,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "audit_version")]
pub enum EpochSignature {
    #[allow(private_interfaces)]
    V1(EpochSignatureV1),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EpochSignatureV1 {
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: i64,
    epoch: Epoch,
    digest: Vec<u8>,
    signature: Vec<u8>,
    key_id: Uuid,
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
    #[error("Verifying key repository error: {0}")]
    VerifyingKeyRepositoryError(#[from] crate::storage::signing_keys::VerifyingKeyRepositoryError),
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    // #[error("Signing error: {0}")]
    // SigningError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializationError),
    // #[error("Signing key repository error: {0}")]
    // SigningKeyRepositoryError(#[from] crate::storage::signing_keys::SigningKeyRepositoryError),
    // #[error("Generic error: {0}")]
    // GenericError(String),
}

impl EpochSignatureV1 {
    fn verify(&self, verifying_key: &VerifyingKey) -> Result<(), VerifyError> {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochSignedMessage {
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: i64,
    epoch: Epoch,
    digest: Vec<u8>,
}

impl EpochSignedMessage {
    pub fn to_vec(&self) -> Result<Vec<u8>, SerializationError> {
        match self.ciphersuite {
            Ciphersuite::ProtobufEd25519 => {
                // Serialize the message to a protobuf format
                // TODO: This is a placeholder; actual serialization logic will depend on the protobuf schema
                Ok(vec![])
            }
            Ciphersuite::JsonEd25519 => {
                // Serialize the message to a JSON format
                Ok(serde_json::to_vec(&self)?)
            }
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

    pub async fn verify(
        &self,
        verifying_key_repo: &impl VerifyingKeyRepository,
    ) -> Result<(), VerifyError> {
        let signing_key_id = self.signing_key_id();
        let verifying_key = verifying_key_repo
            .get_verifying_key(signing_key_id)
            .await?
            .ok_or_else(|| VerifyError::VerifyingKeyNotFound(signing_key_id))?;

        match self {
            EpochSignature::V1(signature) => signature.verify(&verifying_key),
        }
    }
}
