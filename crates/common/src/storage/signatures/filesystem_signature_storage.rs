use crate::{
    epoch_signature::EpochSignature,
    storage::signatures::{SignatureStorage, SignatureStorageError},
};

#[derive(Clone, Debug)]
pub struct FilesystemSignatureStorage {
    root_path: String,
}

impl FilesystemSignatureStorage {
    pub fn new(root_path: String) -> Self {
        FilesystemSignatureStorage { root_path }
    }

    pub fn epoch_path(&self, epoch: &u64) -> String {
        format!("{}/{}", self.root_path, epoch)
    }

    pub fn get_existing_signature_path(&self, epoch: &u64) -> Option<String> {
        let epoch_dir = self.epoch_path(epoch);
        if std::path::Path::new(&epoch_dir).exists() {
            // The signature file is expected to be in the format `<root_hash>.json`
            // so we return the first path we find with the json extension
            let entries = std::fs::read_dir(&epoch_dir).ok()?;
            for entry in entries {
                let path = entry.ok()?.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    return Some(path.to_string_lossy().to_string());
                }
            }
            None
        } else {
            None
        }
    }
}

impl SignatureStorage for FilesystemSignatureStorage {
    async fn has_signature(&self, epoch: &u64) -> Result<bool, SignatureStorageError> {
        let signature_path = self.get_existing_signature_path(epoch);
        match signature_path {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn get_signature(
        &self,
        epoch: &u64,
    ) -> Result<Option<EpochSignature>, SignatureStorageError> {
        let signature_path = self.get_existing_signature_path(epoch);
        if let Some(path) = signature_path {
            let content =
                std::fs::read_to_string(&path).map_err(|e| SignatureStorageError::IoError(e))?;
            let signature: EpochSignature = serde_json::from_str(&content)
                .map_err(|e| SignatureStorageError::SerializationError(e))?;
            Ok(Some(signature))
        } else {
            Ok(None)
        }
    }

    async fn set_signature(
        &mut self,
        epoch: &u64,
        signature: EpochSignature,
    ) -> Result<(), SignatureStorageError> {
        let epoch_dir = self.epoch_path(epoch);

        // ensure the epoch directory is created
        std::fs::create_dir_all(&epoch_dir).map_err(|e| SignatureStorageError::IoError(e))?;

        // Write the signature to a file in the epoch directory
        let signature_path = format!("{}/{}.json", epoch_dir, signature.digest_hex());
        std::fs::write(
            &signature_path,
            serde_json::to_string(&signature)
                .map_err(|e| SignatureStorageError::SerializationError(e))?,
        )
        .map_err(|e| SignatureStorageError::IoError(e))?;

        // Return Ok if everything succeeded
        Ok(())
    }
}
