use serde::{Deserialize, Serialize};

/// Tag identifying which AKD configuration a namespace uses. The concrete
/// configuration types (which depend on the `akd` crate) live in
/// `akd_watch_common::akd_configurations`; here we only carry the variant
/// so the wire DTOs can be parsed by clients that have no need to pull in
/// the akd crate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AkdConfiguration {
    WhatsAppV1Configuration,
    BitwardenV1Configuration,
    #[cfg(any(test, feature = "testing"))]
    TestConfiguration,
}
