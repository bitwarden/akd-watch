use axum::Json;
use serde::{Deserialize, Serialize};
use crate::web::error::ApiError;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "u32")]
#[serde(from = "u32")]
#[repr(u32)]
pub enum Ciphersuite {
    ProtobufEd25519 = 0x0001,
    Unknown(u32),
}

impl From<u32> for Ciphersuite {
    fn from(value: u32) -> Self {
        match value {
            0x0001 => Ciphersuite::ProtobufEd25519,
            other => Ciphersuite::Unknown(other),
        }
    }
}

impl From<Ciphersuite> for u32 {
    fn from(value: Ciphersuite) -> Self {
        match value {
            Ciphersuite::ProtobufEd25519 => 0x0001,
            Ciphersuite::Unknown(other) => other,
        }
    }
}

impl Default for Ciphersuite {
    fn default() -> Self {
        Ciphersuite::ProtobufEd25519
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "u32")]
#[serde(from = "u32")]
#[repr(u32)]
pub enum AuditVersion {
    One = 0x0001,
    Unknown(u32),
}

impl Default for AuditVersion {
    fn default() -> Self {
        AuditVersion::One
    }
}

impl From<u32> for AuditVersion {
    fn from(value: u32) -> Self {
        match value {
            0x0001 => AuditVersion::One,
            other => AuditVersion::Unknown(other),
        }
    }
}

impl From<AuditVersion> for u32 {
    fn from(value: AuditVersion) -> Self {
        match value {
            AuditVersion::One => 0x0001,
            AuditVersion::Unknown(other) => other,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Epoch(u64);

impl Epoch {
    pub fn new(epoch: u64) -> Self {
        Epoch(epoch)
    }
}

impl TryFrom<String> for Epoch {
    type Error = ApiError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value
            .parse::<u64>()
            .map(Epoch)
            .map_err(|e| ApiError::EpochParseError(e))
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SignatureResponse {
    version: AuditVersion,
    ciphersuite: Ciphersuite,
    namespace: String,
    timestamp: u64,
    epoch: Epoch,
    digest: Vec<u8>,
    signature: Vec<u8>,
    key_id: Option<u8>,
    serialized_message: Option<Vec<u8>>,
}

pub async fn handle_audit_query(
    axum::extract::Path((namespace, epoch)): axum::extract::Path<(String, String)>,
) -> Result<Json<SignatureResponse>, ApiError> {
    // Placeholder response
    Ok(Json(SignatureResponse {
        version: AuditVersion::default(),
        ciphersuite: Ciphersuite::default(),
        namespace,
        timestamp: 0,
        epoch: epoch.try_into()?,
        digest: vec![],
        signature: vec![],
        key_id: None,
        serialized_message: None,
    }))
}
