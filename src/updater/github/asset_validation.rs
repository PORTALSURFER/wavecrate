use super::{Release, ReleaseAsset};
use crate::updater::{
    RuntimeIdentity, UpdateChannel, UpdateError, expected_checksums_name,
    expected_checksums_signature_name, expected_zip_asset_name,
};

pub(super) fn find_asset<'a>(release: &'a Release, name: &str) -> Option<&'a ReleaseAsset> {
    release.assets.iter().find(|asset| asset.name == name)
}

pub(super) fn stable_release_has_assets(
    release: &Release,
    version_text: &str,
    identity: &RuntimeIdentity,
) -> Result<bool, UpdateError> {
    let required = stable_required_assets(identity, version_text)?;
    Ok(has_assets(release, &required))
}

pub(super) fn nightly_release_has_assets(
    release: &Release,
    identity: &RuntimeIdentity,
) -> Result<bool, UpdateError> {
    let required = nightly_required_assets(identity)?;
    Ok(has_assets(release, &required))
}

pub(super) fn rc_release_has_assets(
    release: &Release,
    version_text: &str,
    identity: &RuntimeIdentity,
) -> Result<bool, UpdateError> {
    let required = rc_required_assets(identity, version_text)?;
    Ok(has_assets(release, &required))
}

pub(super) fn validate_tagged_release_assets(
    release: &Release,
    tag: &str,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
) -> Result<(), UpdateError> {
    let required = required_assets_for_tag(tag, channel, identity)?;
    if !has_assets(release, &required) {
        return Err(UpdateError::Invalid(format!(
            "Release '{tag}' missing required assets"
        )));
    }
    Ok(())
}

fn required_assets_for_tag(
    tag: &str,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
) -> Result<[String; 3], UpdateError> {
    match channel {
        UpdateChannel::Stable => {
            let version_text = tag.strip_prefix('v').ok_or_else(|| {
                UpdateError::Invalid(format!("Stable tag must start with 'v', got '{tag}'"))
            })?;
            stable_required_assets(identity, version_text)
        }
        UpdateChannel::Rc => {
            let version_text = tag.strip_prefix('v').ok_or_else(|| {
                UpdateError::Invalid(format!("RC tag must start with 'v', got '{tag}'"))
            })?;
            rc_required_assets(identity, version_text)
        }
        UpdateChannel::Nightly => {
            if tag != "nightly" {
                return Err(UpdateError::Invalid(format!(
                    "Nightly tag must be 'nightly', got '{tag}'"
                )));
            }
            nightly_required_assets(identity)
        }
    }
}

fn stable_required_assets(
    identity: &RuntimeIdentity,
    version_text: &str,
) -> Result<[String; 3], UpdateError> {
    Ok([
        expected_zip_asset_name(identity, Some(version_text))?,
        expected_checksums_name(identity, Some(version_text))?,
        expected_checksums_signature_name(identity, Some(version_text))?,
    ])
}

fn nightly_required_assets(identity: &RuntimeIdentity) -> Result<[String; 3], UpdateError> {
    Ok([
        expected_zip_asset_name(identity, None)?,
        expected_checksums_name(identity, None)?,
        expected_checksums_signature_name(identity, None)?,
    ])
}

fn rc_required_assets(
    identity: &RuntimeIdentity,
    version_text: &str,
) -> Result<[String; 3], UpdateError> {
    Ok([
        expected_zip_asset_name(identity, Some(version_text))?,
        expected_checksums_name(identity, Some(version_text))?,
        expected_checksums_signature_name(identity, Some(version_text))?,
    ])
}

fn has_assets(release: &Release, required: &[String; 3]) -> bool {
    required
        .iter()
        .all(|name| find_asset(release, name).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_asset_validation_requires_zip_checksums_and_signature() {
        let identity = identity(UpdateChannel::Stable);
        let release = release_with_assets(
            "v1.2.3",
            false,
            &[
                "wavecrate-1.2.3-windows-x86_64.zip",
                "checksums-1.2.3.txt",
                "checksums-1.2.3.txt.sig",
            ],
        );

        assert!(stable_release_has_assets(&release, "1.2.3", &identity).unwrap());
    }

    #[test]
    fn stable_asset_validation_rejects_missing_signature() {
        let identity = identity(UpdateChannel::Stable);
        let release = release_with_assets(
            "v1.2.3",
            false,
            &["wavecrate-1.2.3-windows-x86_64.zip", "checksums-1.2.3.txt"],
        );

        assert!(!stable_release_has_assets(&release, "1.2.3", &identity).unwrap());
    }

    #[test]
    fn nightly_asset_validation_uses_nightly_contract() {
        let identity = identity(UpdateChannel::Nightly);
        let release = release_with_assets(
            "nightly",
            true,
            &[
                "wavecrate-nightly-windows-x86_64.zip",
                "checksums-nightly.txt",
                "checksums-nightly.txt.sig",
            ],
        );

        assert!(nightly_release_has_assets(&release, &identity).unwrap());
    }

    #[test]
    fn rc_asset_validation_requires_rc_zip_checksums_and_signature() {
        let identity = identity(UpdateChannel::Rc);
        let release = release_with_assets(
            "v1.2.3-rc.2",
            true,
            &[
                "wavecrate-1.2.3-rc.2-windows-x86_64.zip",
                "checksums-1.2.3-rc.2.txt",
                "checksums-1.2.3-rc.2.txt.sig",
            ],
        );

        assert!(rc_release_has_assets(&release, "1.2.3-rc.2", &identity).unwrap());
    }

    fn identity(channel: UpdateChannel) -> RuntimeIdentity {
        RuntimeIdentity {
            app: "wavecrate".to_string(),
            channel,
            target: "target".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
        }
    }

    fn release_with_assets(tag: &str, prerelease: bool, assets: &[&str]) -> Release {
        Release {
            tag_name: tag.to_string(),
            prerelease,
            html_url: "https://example.invalid/release".to_string(),
            published_at: Some("2025-01-01T00:00:00Z".to_string()),
            assets: assets
                .iter()
                .map(|name| ReleaseAsset {
                    name: name.to_string(),
                    browser_download_url: format!("https://example.invalid/{name}"),
                })
                .collect(),
        }
    }
}
