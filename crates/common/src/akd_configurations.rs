use akd::{errors::AkdError, DomainLabel, WhatsAppV1Configuration};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AkdConfiguration {
    WhatsAppV1Configuration,
    BitwardenV1Configuration,
    #[cfg(test)]
    TestConfiguration,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BitwardenV1Label;

impl DomainLabel for BitwardenV1Label {
    fn domain_label() -> &'static [u8] {
        "BitwardenV1Label".as_bytes()
    }
}

pub type BitwardenV1Configuration = akd::ExperimentalConfiguration<BitwardenV1Label>;

#[cfg(test)]
#[derive(Clone, Serialize, Deserialize)]
pub struct TestLabel;

#[cfg(test)]
impl DomainLabel for TestLabel {
    fn domain_label() -> &'static [u8] {
        "TestLabel".as_bytes()
    }
}

#[cfg(test)]
pub type TestAkdConfiguration = akd::ExperimentalConfiguration<TestLabel>;

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
        #[cfg(test)]
        AkdConfiguration::TestConfiguration => {
            akd::auditor::verify_consecutive_append_only::<TestAkdConfiguration>(proof, start_hash, end_hash, end_epoch).await
        }
    }
}
