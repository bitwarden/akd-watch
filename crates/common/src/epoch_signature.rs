use ed25519_dalek::ed25519::signature::SignerMut;
use ed25519_dalek::Verifier;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{crypto::{SigningKey, VerifyingKey}, AkdWatchError, Ciphersuite, Epoch, NamespaceInfo};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "audit_version")]
pub enum EpochSignature {
    V1(EpochSignatureV1),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochSignatureV1 {
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: i64,
    epoch: Epoch,
    digest: Vec<u8>,
    signature: Vec<u8>,
    key_id: Uuid,
}

impl EpochSignatureV1 {
    pub fn from_message(message: EpochSignedMessage, signature: Vec<u8>, key_id: Uuid) -> Self {
        Self {
            ciphersuite: message.ciphersuite,
            namespace: message.namespace,
            timestamp: message.timestamp,
            epoch: message.epoch,
            digest: message.digest,
            signature,
            key_id,
        }
    }

    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<(), AkdWatchError> {
        let message = self.to_message().to_vec()?;

        let signature = ed25519_dalek::Signature::from_bytes(self.signature.as_slice().try_into().map_err(|_| AkdWatchError::SignatureLengthError { expected: 64, actual: self.signature.len() })?);

        if verifying_key.verifying_key.verify(&message.to_vec(), &signature).is_err() {
            return Err(AkdWatchError::SignatureVerificationFailed);
        }

        Ok(())
    }

    pub fn to_message(&self) -> EpochSignedMessage {
        EpochSignedMessage {
            ciphersuite: self.ciphersuite,
            namespace: self.namespace.clone(),
            timestamp: self.timestamp,
            epoch: self.epoch,
            digest: self.digest.clone(),
        }
    }
}

pub struct EpochSignedMessage {
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: i64,
    epoch: Epoch,
    digest: Vec<u8>,
}

impl EpochSignedMessage {
    pub fn to_vec(&self) -> Result<Vec<u8>, AkdWatchError> {
        match self.ciphersuite {
            Ciphersuite::ProtobufEd25519 => {
                // Serialize the message to a protobuf format
                // This is a placeholder; actual serialization logic will depend on the protobuf schema
                Ok(vec![])
            }
            _ => Err(AkdWatchError::UnsupportedCiphersuite(self.ciphersuite))
        }
    }
}

impl EpochSignature {
    pub fn sign(namespace: NamespaceInfo, epoch: Epoch, epoch_root_hash: [u8;32], signing_key: &mut SigningKey) -> Result<Self, AkdWatchError> {
        let message = EpochSignedMessage {
            ciphersuite: Ciphersuite::default(),
            namespace: namespace.name.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            epoch,
            digest: epoch_root_hash.to_vec(),
        };
        let signature = signing_key.signing_key().write().map_err(|_| AkdWatchError::PoisonedSigningKey)?.sign(&message.to_vec()?);
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

    pub fn epoch_root_hash(&self) -> Result<[u8; 32], AkdWatchError> {
        match self {
            EpochSignature::V1(signature) => signature
                .digest
                .as_slice()
                .try_into()
                .map_err(|_| AkdWatchError::EpochRootHashParseError(signature.digest.clone())),
        }
    }
}
