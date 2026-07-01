//! Release asset naming helpers for updater downloads.
//!
//! These helpers keep the GitHub release contract in one place so the apply and
//! discovery paths derive identical asset names for the current runtime.

use super::release_contract;
use super::{APP_NAME, RuntimeIdentity, UpdateChannel, UpdateError};

/// Return the expected zip asset name for a runtime identity.
pub(crate) fn expected_zip_asset_name(
    identity: &RuntimeIdentity,
    version: Option<&str>,
) -> Result<String, UpdateError> {
    let platform = identity.platform.as_str();
    let arch = identity.arch.as_str();
    if !release_contract::supports_platform_arch(platform, arch)? {
        return Err(UpdateError::Invalid(format!(
            "Unsupported platform/arch {}/{}",
            identity.platform, identity.arch
        )));
    }
    match identity.channel {
        UpdateChannel::Stable => {
            let version =
                version.ok_or_else(|| UpdateError::Invalid("Missing stable version".into()))?;
            Ok(format!("{APP_NAME}-{version}-{platform}-{arch}.zip"))
        }
        UpdateChannel::Rc => {
            let version =
                version.ok_or_else(|| UpdateError::Invalid("Missing RC version".into()))?;
            Ok(format!("{APP_NAME}-{version}-{platform}-{arch}.zip"))
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
            Ok(format!("checksums-{version}.txt"))
        }
        UpdateChannel::Rc => {
            let version =
                version.ok_or_else(|| UpdateError::Invalid("Missing RC version".into()))?;
            Ok(format!("checksums-{version}.txt"))
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
            Ok(format!("checksums-{version}.txt.sig"))
        }
        UpdateChannel::Rc => {
            let version =
                version.ok_or_else(|| UpdateError::Invalid("Missing RC version".into()))?;
            Ok(format!("checksums-{version}.txt.sig"))
        }
        UpdateChannel::Nightly => Ok("checksums-nightly.txt.sig".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_zip_asset_name_emits_supported_nightly_assets() {
        assert_eq!(
            expected_zip_asset_name(&identity(UpdateChannel::Nightly, "windows", "x86_64"), None)
                .unwrap(),
            "wavecrate-nightly-windows-x86_64.zip"
        );
        assert_eq!(
            expected_zip_asset_name(&identity(UpdateChannel::Nightly, "macos", "x86_64"), None)
                .unwrap(),
            "wavecrate-nightly-macos-x86_64.zip"
        );
        assert_eq!(
            expected_zip_asset_name(&identity(UpdateChannel::Nightly, "macos", "aarch64"), None)
                .unwrap(),
            "wavecrate-nightly-macos-aarch64.zip"
        );
    }

    #[test]
    fn expected_zip_asset_name_emits_supported_tagged_assets() {
        assert_eq!(
            expected_zip_asset_name(
                &identity(UpdateChannel::Stable, "windows", "x86_64"),
                Some("19.1.0")
            )
            .unwrap(),
            "wavecrate-19.1.0-windows-x86_64.zip"
        );
        assert_eq!(
            expected_zip_asset_name(
                &identity(UpdateChannel::Rc, "macos", "aarch64"),
                Some("19.1.0-rc.2")
            )
            .unwrap(),
            "wavecrate-19.1.0-rc.2-macos-aarch64.zip"
        );
    }

    #[test]
    fn expected_zip_asset_name_rejects_unpublished_release_targets() {
        assert_unsupported("linux", "x86_64");
        assert_unsupported("windows", "aarch64");
        assert_unsupported("macos", "riscv64");
    }

    fn assert_unsupported(platform: &str, arch: &str) {
        let err = expected_zip_asset_name(
            &identity(UpdateChannel::Stable, platform, arch),
            Some("1.2.3"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("Unsupported platform/arch"));
    }

    fn identity(channel: UpdateChannel, platform: &str, arch: &str) -> RuntimeIdentity {
        let identity = RuntimeIdentity {
            app: APP_NAME.to_string(),
            channel,
            target: "target".to_string(),
            platform: platform.to_string(),
            arch: arch.to_string(),
        };
        identity
    }
}
