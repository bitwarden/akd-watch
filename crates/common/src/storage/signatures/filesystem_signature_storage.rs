use tracing::{instrument, trace};

use crate::{
    BINCODE_CONFIG,
    epoch_signature::EpochSignature,
    storage::signatures::{
        SignatureRepository, SignatureRepositoryError, SignatureStorageFileError,
    },
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

    #[instrument(skip_all, fields(epoch))]
    pub fn get_existing_signature_path(&self, epoch: &u64) -> Option<String> {
        let sig_file_path = self.epoch_sig_path(epoch);
        let path = std::path::Path::new(&sig_file_path);
        trace!(
            epoch,
            sig_file_path,
            path_exists = path.exists(),
            path_is_file = path.is_file(),
            "expected signature file path"
        );
        match path.exists() && path.is_file() {
            true => Some(path.to_string_lossy().to_string()),
            false => None,
        }
    }
}

impl SignatureRepository for FilesystemSignatureStorage {
    async fn has_signature(&self, epoch: &u64) -> Result<bool, SignatureRepositoryError> {
        let signature_path = self.get_existing_signature_path(epoch);
        match signature_path {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn get_signature(
        &self,
        epoch: &u64,
    ) -> Result<Option<EpochSignature>, SignatureRepositoryError> {
        let signature_path = self.get_existing_signature_path(epoch);
        trace!(
            epoch,
            signature_path, "Checking for existing signature file"
        );

        if let Some(path) = signature_path {
            trace!(epoch, path, "Found signature file, reading it");
            // Read the signature file to bytes
            let bytes = std::fs::read(&path).map_err(SignatureStorageFileError::IoError)?;
            trace!(
                epoch,
                path,
                "Read {} bytes from signature file",
                bytes.len()
            );

            let signature: EpochSignature = bincode::decode_from_slice(&bytes, BINCODE_CONFIG)?.0;
            trace!(epoch, path, "Decoded signature from file");
            Ok(Some(signature))
        } else {
            trace!(epoch, "No signature file found for epoch");
            Ok(None)
        }
    }

    async fn set_signature(
        &mut self,
        epoch: &u64,
        signature: EpochSignature,
    ) -> Result<(), SignatureRepositoryError> {
        let epoch_dir = self.epoch_path(epoch);

        // ensure the epoch directory is created
        std::fs::create_dir_all(&epoch_dir).map_err(SignatureStorageFileError::IoError)?;

        // Write the signature to a file in the epoch directory
        let signature_path = self.epoch_sig_path(epoch);
        let content = bincode::encode_to_vec(signature, BINCODE_CONFIG)?;
        std::fs::write(&signature_path, content)
            .map_err(SignatureStorageFileError::IoError)?;

        Ok(())
    }
}
