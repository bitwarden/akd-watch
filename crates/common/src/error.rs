use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("bincode serialization error: {0}")]
    BincodeError(#[from] bincode::error::EncodeError),
    #[error("bincode deserialization error: {0}")]
    BincodeDecodeError(#[from] bincode::error::DecodeError),
    #[error("Unsupported format for serialization/deserialization: {0}")]
    UnknownFormat(String),
}
