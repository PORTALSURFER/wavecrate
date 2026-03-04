//! Token storage for issue access, preferring the OS keyring with an opt-in
//! encrypted file fallback when keyring-backed token storage fails.
//! The fallback stores ciphertext on disk while keeping the encryption key in
//! the OS keyring or an explicit environment variable, avoiding recoverable
//! secrets in the filesystem when keyring storage is unavailable.

use crate::app_dirs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Randomness, cipher, and decode helpers for fallback token encryption paths.
mod crypto;
/// Fallback key resolution and caching across env/keyring/legacy file backends.
mod fallback_key;
/// Encrypted fallback token file storage lifecycle and IO hardening.
mod fallback_store;
/// Keyring-backed token/key read-write operations.
mod keyring_backend;

const KEYRING_SERVICE: &str = "sempal";
const KEYRING_KEY: &str = "sempal_github_issue_token";
const FALLBACK_KEYRING_KEY: &str = "sempal_github_issue_token_fallback_key";
const FALLBACK_ALLOW_ENV: &str = "SEMPAL_ALLOW_FALLBACK_TOKEN_STORAGE";
const FALLBACK_KEY_ENV_VAR: &str = "SEMPAL_FALLBACK_KEY";
const MAX_FALLBACK_TOKEN_BYTES: u64 = 16 * 1024;

static FALLBACK_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);
static FALLBACK_KEY_CACHE: OnceLock<Mutex<Option<[u8; 32]>>> = OnceLock::new();

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
/// keyring or be provided via `SEMPAL_FALLBACK_KEY` when keyring storage is
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

fn keyring_disabled() -> bool {
    env_var_truthy("SEMPAL_DISABLE_KEYRING")
}

fn fallback_allowed() -> bool {
    env_var_truthy(FALLBACK_ALLOW_ENV)
}

/// Resolve security-sensitive env toggles using strict tokens only.
///
/// This intentionally accepts only `1` and `true` (ASCII case-insensitive).
/// Unlike the broader shared env parser, we do **not** accept aliases like
/// `yes`/`on` to reduce accidental enablement of keyring-bypass and fallback
/// secret-storage paths.
fn env_var_truthy(key: &str) -> bool {
    std::env::var(key)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn warn_fallback_active() {
    if FALLBACK_WARNING_EMITTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        tracing::warn!(
            "Fallback token storage enabled; ciphertext is stored on disk and the encryption key is stored in the OS keyring or provided via environment."
        );
    }
}

#[cfg(test)]
fn fallback_key_cache() -> &'static Mutex<Option<[u8; 32]>> {
    fallback_key::fallback_key_cache()
}

#[cfg(test)]
fn lock_fallback_key_cache() -> std::sync::MutexGuard<'static, Option<[u8; 32]>> {
    fallback_key::lock_fallback_key_cache()
}

fn random_bytes(len: usize) -> Result<Vec<u8>, IssueTokenStoreError> {
    crypto::random_bytes(len)
}

/// Write a file with restricted permissions using an atomic swap on supported platforms.
fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), IssueTokenStoreError> {
    use std::io::Write;
    let dir = path.parent().ok_or_else(|| {
        IssueTokenStoreError::Io(std::io::Error::other("token path has no parent directory"))
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        IssueTokenStoreError::Io(std::io::Error::other("token path has no file name"))
    })?;

    let mut last_err = None;
    for _ in 0..5 {
        let suffix = random_hex(6)?;
        let tmp_path = dir.join(format!("{}.tmp-{}", file_name.to_string_lossy(), suffix));
        let mut open_options = std::fs::OpenOptions::new();
        open_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open_options.mode(0o600);
        }
        match open_options.open(&tmp_path) {
            Ok(mut file) => {
                file.write_all(bytes)?;
                file.sync_all()?;
                drop(file);
                replace_file(&tmp_path, path)?;
                #[cfg(target_os = "windows")]
                {
                    harden_windows_permissions(path);
                }
                sync_parent_dir(dir)?;
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                last_err = Some(err);
                continue;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Err(IssueTokenStoreError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        format!(
            "failed to create temporary file for {}: {}",
            path.display(),
            last_err
                .as_ref()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "unknown error".into())
        ),
    )))
}

fn replace_file(temp_path: &Path, path: &Path) -> Result<(), IssueTokenStoreError> {
    match std::fs::rename(temp_path, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(target_os = "windows")]
            if err.kind() == std::io::ErrorKind::AlreadyExists
                || err.kind() == std::io::ErrorKind::PermissionDenied
            {
                clear_windows_readonly(path);
                if let Err(e) = std::fs::remove_file(path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Err(e.into());
                    }
                }
                std::fs::rename(temp_path, path)?;
                return Ok(());
            }
            Err(err.into())
        }
    }
}

fn sync_parent_dir(dir: &Path) -> Result<(), IssueTokenStoreError> {
    #[cfg(unix)]
    {
        let dir_handle = std::fs::File::open(dir)?;
        dir_handle.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
    Ok(())
}

fn random_hex(len: usize) -> Result<String, IssueTokenStoreError> {
    let bytes = random_bytes(len)?;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut out, "{:02x}", byte).expect("writing to String should not fail");
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
/// Apply best-effort hiding/readonly attributes for the fallback token file.
/// This is not equivalent to ACLs but avoids a visible plaintext file.
fn harden_windows_permissions(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY, SetFileAttributesW,
        },
        core::PCWSTR,
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let _ = unsafe {
        SetFileAttributesW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_READONLY,
        )
    };
}

#[cfg(target_os = "windows")]
/// Clear readonly attributes so the fallback token file can be replaced.
fn clear_windows_readonly(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, SetFileAttributesW},
        core::PCWSTR,
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let _ = unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_ATTRIBUTE_NORMAL) };
}

fn encrypt(key: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, IssueTokenStoreError> {
    crypto::encrypt(key, nonce, plaintext)
}

fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, IssueTokenStoreError> {
    crypto::decrypt(key, nonce, ciphertext)
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    crypto::decode_hex(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, Once, OnceLock};
    use tempfile::tempdir;

    static MOCK_KEYRING_INIT: Once = Once::new();
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn enable_mock_keyring() {
        MOCK_KEYRING_INIT.call_once(|| {
            keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        });
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn allow_fallback() {
        unsafe {
            std::env::set_var(FALLBACK_ALLOW_ENV, "1");
        }
    }

    fn disallow_fallback() {
        unsafe {
            std::env::remove_var(FALLBACK_ALLOW_ENV);
        }
    }

    fn set_env_key() -> String {
        let env_key = "A".repeat(64);
        unsafe {
            std::env::set_var(FALLBACK_KEY_ENV_VAR, &env_key);
        }
        env_key
    }

    fn clear_env_key() {
        unsafe {
            std::env::remove_var(FALLBACK_KEY_ENV_VAR);
        }
    }

    fn reset_cache() {
        *lock_fallback_key_cache() = None;
    }

    #[test]
    fn fallback_key_cache_recovers_after_poison() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        let _ = std::panic::catch_unwind(|| {
            let _guard = fallback_key_cache()
                .lock()
                .expect("poison fallback key cache");
            panic!("poison fallback key cache");
        });

        store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
        assert_eq!(
            store.get().unwrap().as_deref(),
            Some("tok_abcdefghijklmnopqrstuvwxyz")
        );
        store.delete().unwrap();
        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[test]
    fn fallback_roundtrip_when_keyring_disabled() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();
        assert_eq!(store.get().unwrap(), None);
        store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
        assert_eq!(
            store.get().unwrap().as_deref(),
            Some("tok_abcdefghijklmnopqrstuvwxyz")
        );
        store.delete().unwrap();
        assert_eq!(store.get().unwrap(), None);
        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[test]
    fn set_empty_token_clears_storage() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();
        store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
        store.set("").unwrap();
        assert_eq!(store.get().unwrap(), None);
        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[test]
    fn fallback_is_only_used_when_explicitly_allowed() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        disallow_fallback();
        clear_env_key();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        let base = tempdir().unwrap();
        let _guard = crate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        // Fallback should be disabled unless explicitly allowed.
        let err = store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap_err();
        match err {
            IssueTokenStoreError::Unavailable(message) => {
                assert!(message.contains(FALLBACK_ALLOW_ENV));
            }
            other => panic!("expected unavailable error, got {other:?}"),
        }
        assert!(!store.fallback_token_path().exists());

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        clear_env_key();
    }

    #[test]
    fn fallback_get_rejects_corrupted_payload() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        std::fs::write(store.fallback_token_path(), b"short").unwrap();
        let err = store.fallback_get().unwrap_err();
        match err {
            IssueTokenStoreError::Decode(_) => {}
            other => panic!("expected decode error, got {other:?}"),
        }

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[cfg(unix)]
    #[test]
    fn fallback_token_file_is_private_on_unix() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        use std::os::unix::fs::PermissionsExt;
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
        let token_mode = std::fs::metadata(store.fallback_token_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(token_mode, 0o600);

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[test]
    fn fallback_get_rejects_oversized_payload() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        let oversized = vec![0u8; (MAX_FALLBACK_TOKEN_BYTES + 1) as usize];
        std::fs::write(store.fallback_token_path(), oversized).unwrap();
        let err = store.fallback_get().unwrap_err();
        match err {
            IssueTokenStoreError::Decode(message) => {
                assert!(message.contains("exceeds"));
            }
            other => panic!("expected decode error, got {other:?}"),
        }

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[test]
    fn fallback_get_clears_unreadable_payload() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        let mut payload = vec![0u8; 12];
        payload.extend_from_slice(&[1u8; 16]);
        std::fs::write(store.fallback_token_path(), payload).unwrap();
        assert_eq!(store.fallback_get().unwrap(), None);
        assert!(!store.fallback_token_path().exists());

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[test]
    fn fallback_get_migrates_legacy_key_file() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        allow_fallback();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        let legacy_key_bytes = random_bytes(32).unwrap();
        let mut legacy_key = [0u8; 32];
        legacy_key.copy_from_slice(&legacy_key_bytes);
        write_private_file(&store.legacy_fallback_key_path(), &legacy_key_bytes).unwrap();
        let legacy_payload = store
            .encrypt_fallback_payload(&legacy_key, b"tok_legacy")
            .unwrap();
        write_private_file(&store.fallback_token_path(), &legacy_payload).unwrap();

        // Should successfully read using the file-based key
        assert_eq!(store.fallback_get().unwrap().as_deref(), Some("tok_legacy"));
        // The key file should be removed after migrating to keyring
        assert!(!store.legacy_fallback_key_path().exists());

        disallow_fallback();
    }

    #[test]
    fn fallback_requires_env_key_without_keyring() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        let err = store.set("tok_file_fallback").unwrap_err();
        match err {
            IssueTokenStoreError::Unavailable(message) => {
                assert!(message.contains(FALLBACK_KEY_ENV_VAR));
            }
            other => panic!("expected unavailable error, got {other:?}"),
        }

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
    }

    #[test]
    fn fallback_works_with_env_key() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        reset_cache();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();

        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        store.set("tok_env_fallback").unwrap();
        assert_eq!(store.get().unwrap().as_deref(), Some("tok_env_fallback"));

        // Key file should NOT exist because we provided env var
        assert!(!store.legacy_fallback_key_path().exists());

        store.delete().unwrap();

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    #[test]
    fn fallback_warns_when_active() {
        enable_mock_keyring();
        let _env_guard = env_lock();
        unsafe {
            std::env::set_var("SEMPAL_DISABLE_KEYRING", "1");
        }
        allow_fallback();
        set_env_key();
        FALLBACK_WARNING_EMITTED.store(false, Ordering::SeqCst);
        let base = tempdir().unwrap();
        let _guard = app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
        let store = IssueTokenStore::new().unwrap();

        store.set("tok_abcdefghijklmnopqrstuvwxyz").unwrap();
        assert!(FALLBACK_WARNING_EMITTED.load(Ordering::SeqCst));

        unsafe {
            std::env::remove_var("SEMPAL_DISABLE_KEYRING");
        }
        disallow_fallback();
        clear_env_key();
    }

    /// Security toggles must only accept strict `1`/`true` tokens.
    #[test]
    fn strict_env_var_truthy_only_accepts_one_and_true() {
        let _env_guard = env_lock();
        /// Test-only env var for strict token parsing behavior.
        const STRICT_ENV: &str = "SEMPAL_TOKEN_STORE_STRICT_PARSE_TEST";
        unsafe {
            std::env::remove_var(STRICT_ENV);
        }
        assert!(!env_var_truthy(STRICT_ENV));

        unsafe {
            std::env::set_var(STRICT_ENV, "1");
        }
        assert!(env_var_truthy(STRICT_ENV));

        unsafe {
            std::env::set_var(STRICT_ENV, "TrUe");
        }
        assert!(env_var_truthy(STRICT_ENV));

        unsafe {
            std::env::set_var(STRICT_ENV, "on");
        }
        assert!(!env_var_truthy(STRICT_ENV));

        unsafe {
            std::env::set_var(STRICT_ENV, "yes");
        }
        assert!(!env_var_truthy(STRICT_ENV));

        unsafe {
            std::env::set_var(STRICT_ENV, "true ");
        }
        assert!(!env_var_truthy(STRICT_ENV));

        unsafe {
            std::env::remove_var(STRICT_ENV);
        }
    }
}
