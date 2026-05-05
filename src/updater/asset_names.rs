//! Release asset naming helpers for updater downloads.
//!
//! These helpers keep the GitHub release contract in one place so the apply and
//! discovery paths derive identical asset names for the current runtime.

use super::{APP_NAME, RuntimeIdentity, UpdateChannel, UpdateError};

/// Return the expected zip asset name for a runtime identity.
pub(crate) fn expected_zip_asset_name(
    identity: &RuntimeIdentity,
    version: Option<&str>,
) -> Result<String, UpdateError> {
    let platform = match identity.platform.as_str() {
        "windows" | "linux" | "macos" => identity.platform.as_str(),
        _ => {
            return Err(UpdateError::Invalid(format!(
                "Unsupported platform/arch {}/{}",
                identity.platform, identity.arch
            )));
        }
    };
    let arch = match identity.arch.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => {
            return Err(UpdateError::Invalid(format!(
                "Unsupported platform/arch {}/{}",
                identity.platform, identity.arch
            )));
        }
    };
    match identity.channel {
        UpdateChannel::Stable => {
            let version =
                version.ok_or_else(|| UpdateError::Invalid("Missing stable version".into()))?;
            Ok(format!("{APP_NAME}-v{version}-{platform}-{arch}.zip"))
        }
        UpdateChannel::Nightly => Ok(format!("{APP_NAME}-nightly-{platform}-{arch}.zip")),
    }
}

/// Return the expected checksums file name for a runtime identity.
pub(crate) fn expected_checksums_name(
    identity: &RuntimeIdentity,
    version: Option<&str>,
) -> Result<String, UpdateError> {
    match identity.channel {
        UpdateChannel::Stable => {
            let version =
                version.ok_or_else(|| UpdateError::Invalid("Missing stable version".into()))?;
            Ok(format!("checksums-v{version}.txt"))
        }
        UpdateChannel::Nightly => Ok("checksums-nightly.txt".to_string()),
    }
}

/// Return the expected checksums signature file name for a runtime identity.
pub(crate) fn expected_checksums_signature_name(
    identity: &RuntimeIdentity,
    version: Option<&str>,
) -> Result<String, UpdateError> {
    match identity.channel {
        UpdateChannel::Stable => {
            let version =
                version.ok_or_else(|| UpdateError::Invalid("Missing stable version".into()))?;
            Ok(format!("checksums-v{version}.txt.sig"))
        }
        UpdateChannel::Nightly => Ok("checksums-nightly.txt.sig".to_string()),
    }
}
