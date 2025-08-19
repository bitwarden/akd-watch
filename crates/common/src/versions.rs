use std::{fmt::Display, str::FromStr};

use bincode::{BorrowDecode, Decode, Encode};
use serde::{Deserialize, Serialize};

// Additions to this enum that are not compatible with Plexi should be added
// beginning with 0xF0_01 to avoid conflicts with Plexi versions.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "u32")]
#[serde(from = "u32")]
#[repr(u32)]
pub enum Ciphersuite {
    ProtobufEd25519 = 0x00_01,
    BincodeEd25519 = 0x00_02,
    #[cfg(test)]
    BincodeSpacingTest = 0xF0_00,
    Unknown(u32),
}

impl Encode for Ciphersuite {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        let value: u32 = (*self).into();
        bincode::Encode::encode(&value, encoder)
    }
}

impl<Context> Decode<Context> for Ciphersuite {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let value: u32 = bincode::Decode::decode(decoder)?;
        Ok(value.into())
    }
}

impl<'de, Context> BorrowDecode<'de, Context> for Ciphersuite {
    fn borrow_decode<B: bincode::de::BorrowDecoder<'de, Context = Context>>(
        buffer: &mut B,
    ) -> Result<Self, bincode::error::DecodeError> {
        let value = u32::borrow_decode(buffer)?;
        Ok(value.into())
    }
}

impl From<u32> for Ciphersuite {
    fn from(value: u32) -> Self {
        match value {
            0x00_01 => Ciphersuite::ProtobufEd25519,
            0x00_02 => Ciphersuite::BincodeEd25519,
            #[cfg(test)]
            0xF0_00 => Ciphersuite::BincodeSpacingTest,
            other => Ciphersuite::Unknown(other),
        }
    }
}

impl From<Ciphersuite> for u32 {
    fn from(value: Ciphersuite) -> Self {
        match value {
            Ciphersuite::ProtobufEd25519 => 0x00_01,
            Ciphersuite::BincodeEd25519 => 0x00_02,
            #[cfg(test)]
            Ciphersuite::BincodeSpacingTest => 0xF0_00,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Encode, Decode)]
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

impl FromStr for Epoch {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(Epoch)
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
        value.parse::<u64>().map(Epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ciphersuite_bincode_encode() {
        fn bincode(cs: Ciphersuite) -> Vec<u8> {
            bincode::encode_to_vec(cs, crate::BINCODE_CONFIG).unwrap()
        }

        assert_eq!(bincode(Ciphersuite::ProtobufEd25519), vec![1]);
        assert_eq!(bincode(Ciphersuite::BincodeEd25519), vec![2]);
        assert_eq!(bincode(Ciphersuite::BincodeSpacingTest), vec![251, 0, 240]);
    }

    #[test]
    fn test_ciphersuite_bincode_decode() {
        fn decode(bytes: &[u8]) -> Ciphersuite {
            bincode::decode_from_slice::<Ciphersuite, _>(bytes, crate::BINCODE_CONFIG)
                .unwrap()
                .0
        }

        assert_eq!(decode(&[1]), Ciphersuite::ProtobufEd25519);
        assert_eq!(decode(&[2]), Ciphersuite::BincodeEd25519);
        assert_eq!(decode(&[251, 0, 240]), Ciphersuite::BincodeSpacingTest);
    }
}
