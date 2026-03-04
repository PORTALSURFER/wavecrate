use super::fallback_key::lock_fallback_key_cache;
use super::{
    FALLBACK_ALLOW_ENV, IssueTokenStore, IssueTokenStoreError, MAX_FALLBACK_TOKEN_BYTES, decrypt,
    encrypt, fallback_allowed, random_bytes, warn_fallback_active, write_private_file,
};
use std::path::PathBuf;

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
        if !fallback_allowed() {
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
        warn_fallback_active();
        let key = self.ensure_fallback_key()?;
        let data = std::fs::read(token_path)?;
        if data.len() < 12 {
            return Err(IssueTokenStoreError::Decode("token file too short".into()));
        }
        let (nonce, ciphertext) = data.split_at(12);
        let plaintext = match decrypt(&key, nonce, ciphertext) {
            Ok(plaintext) => plaintext,
            Err(err) => {
                tracing::warn!(
                    "Fallback token payload failed to decrypt; clearing fallback storage: {err}"
                );
                let _ = self.fallback_delete();
                return Ok(None);
            }
        };
        let token = String::from_utf8(plaintext)
            .map_err(|err| IssueTokenStoreError::Decode(err.to_string()))?;
        Ok(Some(token))
    }

    /// Encrypt and store the fallback token payload when fallback storage is enabled.
    pub(super) fn fallback_set(&self, token: &str) -> Result<(), IssueTokenStoreError> {
        if !fallback_allowed() {
            return Err(IssueTokenStoreError::Unavailable(format!(
                "Fallback storage disabled; set {FALLBACK_ALLOW_ENV}=1 to allow encrypted file storage."
            )));
        }
        warn_fallback_active();
        let key = self.ensure_fallback_key()?;
        let payload = self.encrypt_fallback_payload(&key, token.as_bytes())?;
        write_private_file(&self.fallback_token_path(), &payload)?;
        Ok(())
    }

    /// Delete fallback token artifacts and clear the in-memory fallback key cache.
    pub(super) fn fallback_delete(&self) -> Result<(), IssueTokenStoreError> {
        #[cfg(target_os = "windows")]
        {
            clear_windows_readonly(self.fallback_token_path().as_path());
        }
        let _ = std::fs::remove_file(self.fallback_token_path());
        let _ = std::fs::remove_file(self.legacy_fallback_key_path());
        let _ = self.try_keyring_fallback_key_delete();
        *lock_fallback_key_cache() = None;
        Ok(())
    }

    /// Build nonce-prefixed encrypted payload bytes suitable for fallback token persistence.
    pub(super) fn encrypt_fallback_payload(
        &self,
        key: &[u8; 32],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, IssueTokenStoreError> {
        let nonce = random_bytes(12)?;
        let ciphertext = encrypt(key, &nonce, plaintext)?;
        let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);
        Ok(payload)
    }
}

#[cfg(target_os = "windows")]
/// Clear readonly attributes so the fallback token file can be replaced.
fn clear_windows_readonly(path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, SetFileAttributesW},
        core::PCWSTR,
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let _ = unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_ATTRIBUTE_NORMAL) };
}
