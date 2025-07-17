use serde::{Deserialize, Serialize};
use std::future::Future;

use crate::{AkdWatchError, Ciphersuite, Epoch, NamespaceInfo, crypto::SigningKey};

pub trait SignatureStorage {
    fn get_signature(&self, epoch: &u64) -> impl Future<Output = Option<EpochSignature>> + Send;
    fn set_signature(
        &mut self,
        epoch: u64,
        signature: EpochSignature,
    ) -> impl Future<Output = ()> + Send;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "audit_version")]
pub enum EpochSignature {
    V1(EpochSignatureV1),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochSignatureV1 {
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: u64,
    epoch: Epoch,
    digest: Vec<u8>,
    signature: Vec<u8>,
    key_id: Option<u8>,
}

struct EpochSignedMessage {
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: u64,
    epoch: Epoch,
    digest: Vec<u8>,
}

impl EpochSignedMessage {
    pub fn to_vec(&self) -> Vec<u8> {
        match self.ciphersuite {
            Ciphersuite::ProtobufEd25519 => {
                // Serialize the message to a protobuf format
                // This is a placeholder; actual serialization logic will depend on the protobuf schema
                vec![]
            }
            _ => unimplemented!("Unsupported ciphersuite"),
        }
    }
}

impl EpochSignature {
    pub fn sign(namespace: NamespaceInfo, epoch: Epoch, epoch_root_hash: [u8;32], signing_key: &SigningKey) -> Result<Self, AkdWatchError> {
        let signature = signing_key.key.sign()
        Ok(EpochSignature::V1(EpochSignatureV1 { 
            ciphersuite: default(), 
            namespace: namespace.name, 
            timestamp: chrono::Utc::now().timestamp(),
            epoch, 
            digest: epoch_root_hash.into(), 
            signature: (), 
            key_id: ()
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
