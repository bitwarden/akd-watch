use std::fmt::Display;

use akd::{local_auditing::{AuditBlob, AuditBlobName}};

use crate::storage::{AkdStorage, AkdStorageError};

#[derive(Debug, Clone)]
pub struct WhatsAppAkdStorage {
}

impl WhatsAppAkdStorage {
    pub fn new() -> Self {
        WhatsAppAkdStorage {}
    }
}

impl Display for WhatsAppAkdStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WhatsApp AKD")
    }
}

const URL: &str = "http://example.com/blobs";

impl AkdStorage for WhatsAppAkdStorage {
    async fn has_proof(&self, _epoch: u64) -> bool {
        todo!()
    }

    async fn get_proof(&self, name: &AuditBlobName) -> Result<AuditBlob, AkdStorageError> {
        let url = format!("{}/{}", URL, name.to_string());
        let resp = reqwest::get(url).await?.bytes().await?;
        let data = resp.to_vec();
    
        Ok(AuditBlob { data, name: name.clone() })
        }
}
