use super::{
    FALLBACK_KEYRING_KEY, IssueTokenStore, IssueTokenStoreError, KEYRING_KEY, KEYRING_SERVICE,
    keyring_disabled,
};
use base64::Engine as _;

impl IssueTokenStore {
    /// Read the primary issue token from the OS keyring when keyring access is enabled.
    pub(super) fn try_keyring_get(&self) -> Result<Option<String>, IssueTokenStoreError> {
        if keyring_disabled() {
            return Ok(None);
        }
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_KEY)
            .map_err(|err| IssueTokenStoreError::Unavailable(err.to_string()))?;
        match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(IssueTokenStoreError::Unavailable(err.to_string())),
        }
    }

    /// Persist the primary issue token in the OS keyring.
    pub(super) fn try_keyring_set(&self, token: &str) -> Result<(), IssueTokenStoreError> {
        if keyring_disabled() {
            return Err(IssueTokenStoreError::Unavailable("keyring disabled".into()));
        }
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_KEY)
            .map_err(|err| IssueTokenStoreError::Unavailable(err.to_string()))?;
        entry
            .set_password(token)
            .map_err(|err| IssueTokenStoreError::Unavailable(err.to_string()))
    }

    /// Remove the primary issue token from the OS keyring.
    pub(super) fn try_keyring_delete(&self) -> Result<(), IssueTokenStoreError> {
        if keyring_disabled() {
            return Ok(());
        }
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_KEY)
            .map_err(|err| IssueTokenStoreError::Unavailable(err.to_string()))?;
        let _ = entry.delete_credential();
        Ok(())
    }

    /// Open the keyring entry used to persist the fallback encryption key.
    pub(super) fn fallback_key_entry(&self) -> Result<keyring::Entry, IssueTokenStoreError> {
        keyring::Entry::new(KEYRING_SERVICE, FALLBACK_KEYRING_KEY)
            .map_err(|err| IssueTokenStoreError::Unavailable(err.to_string()))
    }

    /// Read the fallback encryption key from the OS keyring.
    pub(super) fn try_keyring_fallback_key_get(
        &self,
    ) -> Result<Option<[u8; 32]>, IssueTokenStoreError> {
        if keyring_disabled() {
            return Ok(None);
        }
        let entry = self.fallback_key_entry()?;
        match entry.get_password() {
            Ok(encoded) => {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|err| IssueTokenStoreError::Decode(err.to_string()))?;
                if decoded.len() != 32 {
                    return Ok(None);
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(&decoded);
                Ok(Some(key))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(IssueTokenStoreError::Unavailable(format!(
                "Fallback keyring unavailable ({err})."
            ))),
        }
    }

    /// Persist the fallback encryption key in the OS keyring.
    pub(super) fn try_keyring_fallback_key_set(
        &self,
        key: &[u8; 32],
    ) -> Result<(), IssueTokenStoreError> {
        if keyring_disabled() {
            return Err(IssueTokenStoreError::Unavailable("keyring disabled".into()));
        }
        let entry = self.fallback_key_entry()?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);
        entry
            .set_password(&encoded)
            .map_err(|err| IssueTokenStoreError::Unavailable(err.to_string()))
    }

    /// Remove the fallback encryption key from the OS keyring.
    pub(super) fn try_keyring_fallback_key_delete(&self) -> Result<(), IssueTokenStoreError> {
        if keyring_disabled() {
            return Ok(());
        }
        let entry = self.fallback_key_entry()?;
        let _ = entry.delete_credential();
        Ok(())
    }
}
