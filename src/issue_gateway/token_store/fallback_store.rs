use super::{
    FALLBACK_ALLOW_ENV, IssueTokenStore, IssueTokenStoreError, MAX_FALLBACK_TOKEN_BYTES,
    cleanup_result, crypto, fallback_key, fallback_policy, file_io,
};
use std::path::{Path, PathBuf};

impl IssueTokenStore {
    /// Return the encrypted fallback token payload path under the per-user secrets dir.
    pub(super) fn fallback_token_path(&self) -> PathBuf {
        self.fallback_dir.join("github_issue_token.bin")
    }

    /// Return the legacy on-disk fallback key path used for migration and cleanup.
    pub(super) fn legacy_fallback_key_path(&self) -> PathBuf {
        self.fallback_dir.join("encryption.key")
    }

    /// Read and decrypt the fallback token payload when fallback storage is enabled.
    pub(super) fn fallback_get(&self) -> Result<Option<String>, IssueTokenStoreError> {
        if !fallback_policy::fallback_allowed() {
            return Err(IssueTokenStoreError::Unavailable(format!(
                "Fallback storage disabled; set {FALLBACK_ALLOW_ENV}=1 to allow encrypted file storage."
            )));
        }
        let token_path = self.fallback_token_path();
        if !token_path.exists() {
            return Ok(None);
        }
        let metadata = std::fs::metadata(&token_path)?;
        if metadata.len() > MAX_FALLBACK_TOKEN_BYTES {
            return Err(IssueTokenStoreError::Decode(format!(
                "fallback token file exceeds {MAX_FALLBACK_TOKEN_BYTES} bytes"
            )));
        }
        fallback_policy::warn_fallback_active();
        let key = self.ensure_fallback_key()?;
        let data = std::fs::read(token_path)?;
        if data.len() < 12 {
            return Err(IssueTokenStoreError::Decode("token file too short".into()));
        }
        let (nonce, ciphertext) = data.split_at(12);
        let plaintext = match crypto::decrypt(&key, nonce, ciphertext) {
            Ok(plaintext) => plaintext,
            Err(err) => {
                tracing::warn!(
                    "Fallback token payload failed to decrypt; clearing fallback storage: {err}"
                );
                self.fallback_delete()?;
                return Ok(None);
            }
        };
        let token = String::from_utf8(plaintext)
            .map_err(|err| IssueTokenStoreError::Decode(err.to_string()))?;
        Ok(Some(token))
    }

    /// Encrypt and store the fallback token payload when fallback storage is enabled.
    pub(super) fn fallback_set(&self, token: &str) -> Result<(), IssueTokenStoreError> {
        if !fallback_policy::fallback_allowed() {
            return Err(IssueTokenStoreError::Unavailable(format!(
                "Fallback storage disabled; set {FALLBACK_ALLOW_ENV}=1 to allow encrypted file storage."
            )));
        }
        fallback_policy::warn_fallback_active();
        let key = self.ensure_fallback_key()?;
        let payload = self.encrypt_fallback_payload(&key, token.as_bytes())?;
        file_io::write_private_file(&self.fallback_token_path(), &payload)?;
        Ok(())
    }

    /// Delete fallback token artifacts and clear the in-memory fallback key cache.
    pub(super) fn fallback_delete(&self) -> Result<(), IssueTokenStoreError> {
        #[cfg(target_os = "windows")]
        {
            file_io::clear_windows_readonly(self.fallback_token_path().as_path());
        }
        let mut failures = Vec::new();
        remove_optional_file(
            self.fallback_token_path().as_path(),
            "fallback token",
            &mut failures,
        );
        remove_optional_file(
            self.legacy_fallback_key_path().as_path(),
            "legacy fallback key",
            &mut failures,
        );
        if let Err(err) = self.try_keyring_fallback_key_delete() {
            failures.push(super::TokenCleanupFailure {
                artifact: "fallback keyring key",
                path: None,
                message: err.to_string(),
            });
        }
        *fallback_key::lock_fallback_key_cache() = None;
        cleanup_result(failures)
    }

    /// Build nonce-prefixed encrypted payload bytes suitable for fallback token persistence.
    pub(super) fn encrypt_fallback_payload(
        &self,
        key: &[u8; 32],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, IssueTokenStoreError> {
        let nonce = crypto::random_bytes(12)?;
        let ciphertext = crypto::encrypt(key, &nonce, plaintext)?;
        let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);
        Ok(payload)
    }
}

pub(super) fn remove_optional_file(
    path: &Path,
    artifact: &'static str,
    failures: &mut Vec<super::TokenCleanupFailure>,
) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => failures.push(super::TokenCleanupFailure {
            artifact,
            path: Some(path.to_path_buf()),
            message: err.to_string(),
        }),
    }
}
