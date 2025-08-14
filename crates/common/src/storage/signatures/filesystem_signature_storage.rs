use crate::{
    epoch_signature::EpochSignature,
    storage::signatures::{SignatureStorage, SignatureStorageError, SignatureStorageFileError}, BINCODE_CONFIG,
};

#[derive(Clone, Debug)]
pub struct FilesystemSignatureStorage {
    root_path: String,
}

const SIG_FILE_NAME: &str = "sig";

impl FilesystemSignatureStorage {
    pub fn new(root_path: String) -> Self {
        FilesystemSignatureStorage { root_path }
    }

    pub fn epoch_path(&self, epoch: &u64) -> String {
        format!("{}/{}", self.root_path, epoch)
    }

    pub fn epoch_sig_path(&self, epoch: &u64) -> String {
        format!("{}/{}/{}", self.root_path, epoch, SIG_FILE_NAME)
    }

    pub fn get_existing_signature_path(&self, epoch: &u64) -> Option<String> {
        let sig_file_path = self.epoch_sig_path(epoch);
        let path = std::path::Path::new(&sig_file_path);
        match path.exists() && path.is_file() {
            true => Some(path.to_string_lossy().to_string()),
            false => None,
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
            // Read the signature file to bytes
            let bytes = std::fs::read(&path)
                .map_err(|e| SignatureStorageFileError::IoError(e))?;

            let signature: EpochSignature = bincode::decode_from_slice(&bytes, BINCODE_CONFIG)?.0;
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
        std::fs::create_dir_all(&epoch_dir).map_err(|e| SignatureStorageFileError::IoError(e))?;

        // Write the signature to a file in the epoch directory
        let signature_path = self.epoch_sig_path(epoch);
        let content = bincode::encode_to_vec(signature, BINCODE_CONFIG)?;
        std::fs::write(
            &signature_path,
            content,
        )
        .map_err(|e| SignatureStorageFileError::IoError(e))?;

        Ok(())
    }
}
