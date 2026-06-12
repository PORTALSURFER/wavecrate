//! Token storage for issue access, preferring the OS keyring with an opt-in
//! encrypted file fallback when keyring-backed token storage fails.
//! The fallback stores ciphertext on disk while keeping the encryption key in
//! the OS keyring or an explicit environment variable, avoiding recoverable
//! secrets in the filesystem when keyring storage is unavailable.

use crate::app_dirs;
use std::path::PathBuf;

/// Randomness, cipher, and decode helpers for fallback token encryption paths.
mod crypto;
/// Fallback key resolution and caching across env/keyring/legacy file backends.
mod fallback_key;
/// Fallback enablement, keyring-disable gates, and warning policy.
mod fallback_policy;
/// Encrypted fallback token file storage lifecycle and IO hardening.
mod fallback_store;
/// Private fallback-token file replacement and platform permission helpers.
mod file_io;
/// Keyring-backed token/key read-write operations.
mod keyring_backend;

use fallback_policy::fallback_allowed;

const KEYRING_SERVICE: &str = "wavecrate";
const KEYRING_KEY: &str = "wavecrate_github_issue_token";
const FALLBACK_KEYRING_KEY: &str = "wavecrate_github_issue_token_fallback_key";
const FALLBACK_ALLOW_ENV: &str = "WAVECRATE_ALLOW_FALLBACK_TOKEN_STORAGE";
const FALLBACK_KEY_ENV_VAR: &str = "WAVECRATE_FALLBACK_KEY";
const MAX_FALLBACK_TOKEN_BYTES: u64 = 16 * 1024;

/// Errors returned by the issue token storage backend.
#[derive(Debug, thiserror::Error)]
pub enum IssueTokenStoreError {
    /// Token storage is unavailable on this system.
    #[error("Token store unavailable: {0}")]
    Unavailable(String),
    /// IO error while reading/writing storage.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Crypto error when encrypting or decrypting.
    #[error("Crypto error: {0}")]
    Crypto(String),
    /// Decode error when parsing stored values.
    #[error("Decode error: {0}")]
    Decode(String),
    /// Failed to resolve app directories.
    #[error("App dir error: {0}")]
    AppDir(#[from] crate::app_dirs::AppDirError),
}

/// Stores the issue token in the OS keyring with an opt-in encrypted file fallback.
///
/// The fallback stores ciphertext on disk. The encryption key must live in the OS
/// keyring or be provided via `WAVECRATE_FALLBACK_KEY` when keyring storage is
/// unavailable.
#[derive(Clone, Debug)]
pub struct IssueTokenStore {
    fallback_dir: PathBuf,
}

impl IssueTokenStore {
    /// Create a token store rooted in the configured app directory.
    pub fn new() -> Result<Self, IssueTokenStoreError> {
        let fallback_dir = app_dirs::app_root_dir()?.join("secrets");
        std::fs::create_dir_all(&fallback_dir)?;
        Ok(Self { fallback_dir })
    }

    /// Load the token from the keyring or the opt-in fallback storage if allowed.
    pub fn get(&self) -> Result<Option<String>, IssueTokenStoreError> {
        match self.try_keyring_get() {
            Ok(Some(token)) => Ok(Some(token)),
            Ok(None) => {
                if fallback_allowed() {
                    self.fallback_get()
                } else {
                    Ok(None)
                }
            }
            Err(keyring_err) => {
                if fallback_allowed() {
                    // Keyring failed, try fallback if explicitly enabled.
                    self.fallback_get()
                } else {
                    Err(IssueTokenStoreError::Unavailable(format!(
                        "Keyring unavailable ({keyring_err}). Fallback storage is disabled; set {FALLBACK_ALLOW_ENV}=1 to allow encrypted file storage."
                    )))
                }
            }
        }
    }

    /// Store the token, preferring the OS keyring and using the fallback only
    /// when explicitly enabled.
    pub fn set(&self, token: &str) -> Result<(), IssueTokenStoreError> {
        let token = token.trim();
        if token.is_empty() {
            return self.delete();
        }

        let keyring_err = match self.try_keyring_set(token) {
            Ok(_) => {
                // Verify it can be read back - with retries for flaky backends
                let mut last_error = None;
                for _ in 0..5 {
                    match self.try_keyring_get() {
                        Ok(Some(stored)) if stored == token => {
                            let _ = self.fallback_delete();
                            return Ok(());
                        }
                        Ok(Some(stored)) => {
                            last_error = Some(IssueTokenStoreError::Unavailable(format!(
                                "Keyring set succeeded but read back mismatch (got {} bytes, expected {}).",
                                stored.len(),
                                token.len()
                            )));
                        }
                        Ok(None) => {
                            last_error = Some(IssueTokenStoreError::Unavailable(
                                "Keyring set reported success but item was not found immediately after.".into(),
                            ));
                        }
                        Err(e) => {
                            last_error = Some(e);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }

                // If we get here, keyring failed after retries. Use fallback automatically.
                last_error
            }
            Err(e) => Some(e),
        };

        if fallback_allowed() {
            // Keyring failed, use fallback storage only when explicitly enabled.
            match self.fallback_set(token) {
                Ok(_) => Ok(()),
                Err(fallback_err) => Err(fallback_err),
            }
        } else {
            let keyring_error = keyring_err
                .as_ref()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "unknown keyring error".into());
            Err(IssueTokenStoreError::Unavailable(format!(
                "Keyring unavailable ({keyring_error}). Fallback storage is disabled; set {FALLBACK_ALLOW_ENV}=1 to allow encrypted file storage."
            )))
        }
    }

    /// Store the token and verify it can be read back.
    pub fn set_and_verify(&self, token: &str) -> Result<(), IssueTokenStoreError> {
        self.set(token)
    }

    /// Remove the token from all storage backends.
    pub fn delete(&self) -> Result<(), IssueTokenStoreError> {
        let _ = self.try_keyring_delete();
        let _ = self.fallback_delete();
        Ok(())
    }
}

#[cfg(test)]
/// Issue-token store behavior tests.
mod tests;
