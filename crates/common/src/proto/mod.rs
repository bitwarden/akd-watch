pub mod types {
    include!(concat!(env!("OUT_DIR"), "/akd_watch_common.types.rs"));
}

#[derive(Debug, thiserror::Error)]
#[error("A protobuf conversion error occurred")]
pub struct ConversionError;

impl From<&crate::Epoch> for crate::proto::types::Epoch {
    fn from(input: &crate::Epoch) -> Self {
        Self {
            inner: *input.value(),
        }
    }
}

impl From<&crate::proto::types::Epoch> for crate::Epoch {
    fn from(input: &crate::proto::types::Epoch) -> Self {
        Self::new(input.inner)
    }
}

impl From<&crate::EpochSignedMessage> for crate::proto::types::SignatureMessage {
    fn from(input: &crate::EpochSignedMessage) -> Self {
        Self {
            ciphersuite: input.ciphersuite.into(),
            namespace: input.namespace.clone(),
            timestamp: input.timestamp as u64,
            epoch: (&input.epoch).into(),
            digest: input.digest.clone(),
        }
    }
}

impl TryFrom<crate::proto::types::SignatureMessage> for crate::EpochSignedMessage {
    type Error = ConversionError;

    fn try_from(input: crate::proto::types::SignatureMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            ciphersuite: input.ciphersuite.into(),
            namespace: input.namespace.clone(),
            timestamp: input.timestamp as i64,
            epoch: (&input.epoch).into(),
            digest: input.digest.clone(),
        })
    }
}
