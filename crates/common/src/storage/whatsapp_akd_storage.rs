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

    async fn get_proof_name(&self, _epoch: u64) -> Result<AuditBlobName, AkdStorageError> {
        // TODO: reqwest this from a real URL
        AuditBlobName::try_from("458298/5f02bf9c5526151669914c4b80a300870e583b6b32e2c537ee4fa4f589fe889d/3ae9497069cc722dc9e00f8251da87071646a57dae2fc7882f1d8214961d80bd")
            .map_err(|_| AkdStorageError::Custom("Invalid blob name format".to_string()))
    }
}
