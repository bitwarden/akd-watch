use akd::{errors::AkdError, DomainLabel, WhatsAppV1Configuration};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AkdConfiguration {
    WhatsAppV1Configuration,
    BitwardenV1Configuration
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BitwardenV1Label;

impl DomainLabel for BitwardenV1Label {
    fn domain_label() -> &'static [u8] {
        "BitwardenV1Label".as_bytes()
    }
}

pub type BitwardenV1Configuration = akd::ExperimentalConfiguration<BitwardenV1Label>;

/// Helper function to verify consecutive append-only proofs for a given configuration.
pub async fn verify_consecutive_append_only(
    configuration: &AkdConfiguration,
    proof: &akd::SingleAppendOnlyProof,
    start_hash: [u8; 32],
    end_hash: [u8; 32],
    end_epoch: u64,
) -> Result<(), AkdError> {
    match configuration {
        AkdConfiguration::WhatsAppV1Configuration => {
            akd::auditor::verify_consecutive_append_only::<WhatsAppV1Configuration>(proof, start_hash, end_hash, end_epoch).await
        }
        AkdConfiguration::BitwardenV1Configuration => {
            akd::auditor::verify_consecutive_append_only::<BitwardenV1Configuration>(proof, start_hash, end_hash, end_epoch).await
        }
    }
}
