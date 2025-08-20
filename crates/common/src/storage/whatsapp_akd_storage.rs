use std::fmt::Display;

use akd::local_auditing::{AuditBlob, AuditBlobName};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::header::CACHE_CONTROL;
use tracing::instrument;

use crate::storage::{AkdProofDirectoryError, AkdProofNameError, AkdStorage};

#[derive(Debug, Clone)]
pub struct WhatsAppAkdStorage {
    base_url: String,
}

impl Default for WhatsAppAkdStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl WhatsAppAkdStorage {
    pub fn new() -> Self {
        WhatsAppAkdStorage {
            base_url: "https://d1tfr3x7n136ak.cloudfront.net".to_string(),
        }
    }

    #[cfg(test)]
    pub fn new_with_url(base_url: String) -> Self {
        WhatsAppAkdStorage { base_url }
    }
}

impl Display for WhatsAppAkdStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WhatsApp AKD")
    }
}

impl WhatsAppAkdStorage {
    async fn get_key_for_epoch(
        &self,
        epoch: &u64,
    ) -> Result<Option<String>, AkdProofDirectoryError> {
        let url = format!("{}/?list-type=2&prefix={}/", self.base_url, epoch);
        // make a client with no chache
        let client = reqwest::Client::new();
        // TODO: we're getting proofs that are delayed by minutes vs cloudflare's dashboard. Need to figure out why we're so far behind
        let resp = client
            .get(url)
            .header(CACHE_CONTROL, "no-store")
            .send()
            .await?
            .bytes()
            .await?;

        let mut reader = Reader::from_reader(resp.as_ref());
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"Key" => {
                    // Read the key content
                    if let Ok(Event::Text(e)) = reader.read_event_into(&mut buf) {
                        let key_text = std::str::from_utf8(e.as_ref())?;
                        return Ok(Some(key_text.to_string()));
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(e)?,
                _ => (),
            }
            buf.clear();
        }

        Ok(None)
    }
}

impl AkdStorage for WhatsAppAkdStorage {
    #[instrument(level = "info", skip_all, fields(base_url = self.base_url, epoch = epoch))]
    async fn has_proof(&self, epoch: &u64) -> bool {
        self.get_key_for_epoch(epoch)
            .await
            .map(|key| key.is_some())
            .unwrap_or(false)
    }

    #[instrument(level = "info", skip_all, fields(base_url = self.base_url, epoch = name.epoch))]
    async fn get_proof(&self, name: &AuditBlobName) -> Result<AuditBlob, AkdProofDirectoryError> {
        let url = format!("{}/{}", self.base_url, name.to_string());
        let resp = reqwest::get(url).await?.bytes().await?;
        let data = resp.to_vec();

        Ok(AuditBlob {
            data,
            name: *name,
        })
    }

    #[instrument(level = "info", skip_all, fields(base_url = self.base_url, epoch = epoch))]
    async fn get_proof_name(&self, epoch: &u64) -> Result<AuditBlobName, AkdProofNameError> {
        match self.get_key_for_epoch(epoch).await? {
            Some(key) => AuditBlobName::try_from(key.as_str())
                .map_err(|_| AkdProofNameError::AuditBlobNameParsingError),
            None => Err(AkdProofNameError::ProofNotFound(*epoch)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    const EPOCH_KEY: &str = "1381400/6a05c589fb2c47aed2d03a731974c7b8ddedfc11aa504f003d60b284f97ef78f/2a60babcf966b100f71c13f76e708bf84ba12d777a7d90a0b8587c56f9bf4016";
    const TEST_EPOCH: &u64 = &1381400;

    fn create_xml_response_with_key(key: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>kt-audit-proofs-integration-v2</Name>
  <Prefix></Prefix>
  <Marker></Marker>
  <MaxKeys>1000</MaxKeys>
  <IsTruncated>false</IsTruncated>
  <Contents>
    <Key>{key}</Key>
    <LastModified>2023-01-01T00:00:00.000Z</LastModified>
    <ETag>"abcd1234"</ETag>
    <Size>1024</Size>
    <StorageClass>STANDARD</StorageClass>
  </Contents>
</ListBucketResult>"#
        )
    }

    fn create_empty_xml_response() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>kt-audit-proofs-integration-v2</Name>
  <Prefix></Prefix>
  <Marker></Marker>
  <MaxKeys>1000</MaxKeys>
  <IsTruncated>false</IsTruncated>
</ListBucketResult>"#
            .to_string()
    }

    #[tokio::test]
    async fn test_has_proof_existing_epoch() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/?list-type=2&prefix=1381400/")
            .with_status(200)
            .with_body(create_xml_response_with_key(EPOCH_KEY))
            .create_async()
            .await;

        let storage = WhatsAppAkdStorage::new_with_url(server.url());
        let result = storage.has_proof(TEST_EPOCH).await;

        mock.assert_async().await;
        assert!(result, "Epoch should exist");
    }

    #[tokio::test]
    async fn test_has_proof_nonexistent_epoch() {
        let mut server = mockito::Server::new_async().await;
        let nonexistent_epoch = &999999999999u64;
        let mock = server
            .mock("GET", "/?list-type=2&prefix=999999999999/")
            .with_status(200)
            .with_body(create_empty_xml_response())
            .create_async()
            .await;

        let storage = WhatsAppAkdStorage::new_with_url(server.url());
        let result = storage.has_proof(nonexistent_epoch).await;

        mock.assert_async().await;
        assert!(!result, "Nonexistent epoch should not exist");
    }

    #[tokio::test]
    async fn test_get_key_for_epoch_existing() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/?list-type=2&prefix=1381400/")
            .with_status(200)
            .with_body(create_xml_response_with_key(EPOCH_KEY))
            .create_async()
            .await;

        let storage = WhatsAppAkdStorage::new_with_url(server.url());
        match storage.get_key_for_epoch(TEST_EPOCH).await {
            Ok(Some(key)) => {
                mock.assert_async().await;
                assert_eq!(key, EPOCH_KEY, "Key should match expected value");
            }
            Ok(None) => panic!("Key should be present"),
            Err(e) => panic!("Error checking epoch: {e}"),
        }
    }

    #[tokio::test]
    async fn test_get_key_for_epoch_nonexistent() {
        let mut server = mockito::Server::new_async().await;
        let nonexistent_epoch = &999999999999u64;
        let mock = server
            .mock("GET", "/?list-type=2&prefix=999999999999/")
            .with_status(200)
            .with_body(create_empty_xml_response())
            .create_async()
            .await;

        let storage = WhatsAppAkdStorage::new_with_url(server.url());
        match storage.get_key_for_epoch(nonexistent_epoch).await {
            Ok(None) => {
                mock.assert_async().await;
                // Expected - no key found
            }
            Ok(Some(_)) => panic!("Should not find key for nonexistent epoch"),
            Err(e) => panic!("Error checking epoch: {e}"),
        }
    }

    #[tokio::test]
    async fn test_get_proof_name_existing() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/?list-type=2&prefix=1381400/")
            .with_status(200)
            .with_body(create_xml_response_with_key(EPOCH_KEY))
            .create_async()
            .await;

        let storage = WhatsAppAkdStorage::new_with_url(server.url());
        match storage.get_proof_name(TEST_EPOCH).await {
            Ok(name) => {
                mock.assert_async().await;
                assert_eq!(
                    name.to_string(),
                    EPOCH_KEY,
                    "Proof name should match expected key"
                );
            }
            Err(e) => panic!("Error getting proof name: {e}"),
        }
    }

    #[tokio::test]
    async fn test_get_proof_name_nonexistent() {
        let mut server = mockito::Server::new_async().await;
        let nonexistent_epoch = &999999999999u64;
        let mock = server
            .mock("GET", "/?list-type=2&prefix=999999999999/")
            .with_status(200)
            .with_body(create_empty_xml_response())
            .create_async()
            .await;

        let storage = WhatsAppAkdStorage::new_with_url(server.url());
        match storage.get_proof_name(nonexistent_epoch).await {
            Ok(_) => panic!("Should not find proof for nonexistent epoch"),
            Err(e) => {
                mock.assert_async().await;
                let error_message = format!("{e}");
                assert!(
                    error_message
                        .contains(&format!("Proof not found for epoch {nonexistent_epoch}")),
                    "Error message should indicate epoch not found: {error_message}"
                );
            }
        }
    }
}
