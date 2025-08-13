use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("Serde JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Unsupported format for serialization/deserialization: {0}")]
    UnknownFormat(String),
}
