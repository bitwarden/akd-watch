use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "u32")]
#[serde(from = "u32")]
#[repr(u32)]
pub enum Ciphersuite {
    ProtobufEd25519 = 0x0001,
    JsonEd25519 = 0xF000,
    Unknown(u32),
}

impl From<u32> for Ciphersuite {
    fn from(value: u32) -> Self {
        match value {
            0x0001 => Ciphersuite::ProtobufEd25519,
            0xF000 => Ciphersuite::JsonEd25519,
            other => Ciphersuite::Unknown(other),
        }
    }
}

impl From<Ciphersuite> for u32 {
    fn from(value: Ciphersuite) -> Self {
        match value {
            Ciphersuite::ProtobufEd25519 => 0x0001,
            Ciphersuite::JsonEd25519 => 0xF000,
            Ciphersuite::Unknown(other) => other,
        }
    }
}

impl Default for Ciphersuite {
    fn default() -> Self {
        // TODO: Should we stick with protobuf like plexi?
        Ciphersuite::JsonEd25519
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

    pub fn value(&self) -> &u64 {
        &self.0
    }

    pub fn next(&self) -> Self {
        Epoch(self.0 + 1)
    }
}

impl From<u64> for Epoch {
    fn from(value: u64) -> Self {
        Epoch(value)
    }
}

impl From<Epoch> for u64 {
    fn from(epoch: Epoch) -> Self {
        epoch.0
    }
}

impl Display for Epoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Epoch {
    type Error = std::num::ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value
            .parse::<u64>()
            .map(Epoch)
    }
}
