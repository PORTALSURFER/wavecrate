//! Public PortalSurfer release-catalog checks.

use semver::Version;
use serde::Deserialize;

use crate::http_client;

use super::{UpdateChannel, UpdateError, release_contract};

const MAX_RELEASE_CATALOG_JSON_BYTES: usize = 512 * 1024;
/// Public Wavecrate release catalog exposed by portalsurfer.org.
pub const PUBLIC_RELEASE_CATALOG_URL: &str = "https://portalsurfer.org/wavecrate/api/v1/releases";
/// Human-facing Wavecrate download page on portalsurfer.org.
pub const PUBLIC_RELEASE_PAGE_URL: &str = "https://portalsurfer.org/wavecrate/";

/// Request for checking the public download catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicReleaseCheckRequest {
    /// Catalog URL to read.
    pub catalog_url: String,
    /// Current running build number.
    pub current_build_number: u64,
    /// Current running git SHA.
    pub current_build_sha: String,
    /// Runtime platform label used in release file names.
    pub platform: String,
    /// Runtime architecture label used in release file names.
    pub arch: String,
    /// Update channel to select from the public catalog.
    pub channel: UpdateChannel,
}

impl PublicReleaseCheckRequest {
    /// Build a request for the current binary and target.
    pub fn current(current_build_number: u64, channel: UpdateChannel) -> Self {
        Self {
            catalog_url: PUBLIC_RELEASE_CATALOG_URL.to_string(),
            current_build_number,
            current_build_sha: env!("WAVECRATE_BUILD_GIT_SHA").to_string(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            channel,
        }
    }
}

/// Public release information surfaced by update indicators.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicReleaseInfo {
    /// Portal release build id.
    pub build_id: String,
    /// Numeric Wavecrate build number.
    pub build_number: u64,
    /// Version label published by the release catalog.
    pub version: String,
    /// Release timestamp from the public catalog.
    pub released_at: String,
    /// Public download-page URL.
    pub download_page_url: String,
}

/// Check the public PortalSurfer release catalog for a newer usable release.
pub fn check_public_release_catalog(
    request: PublicReleaseCheckRequest,
) -> Result<Option<PublicReleaseInfo>, UpdateError> {
    let response = http_client::agent()
        .get(&request.catalog_url)
        .set("User-Agent", "wavecrate-update-indicator")
        .set("Accept", "application/json")
        .call()
        .map_err(map_http_error)?;
    let bytes = http_client::read_response_bytes(response, MAX_RELEASE_CATALOG_JSON_BYTES)
        .map_err(|err| UpdateError::Http(err.to_string()))?;
    let catalog: PublicReleaseCatalog = serde_json::from_slice(&bytes)?;
    latest_available_public_release(
        &catalog,
        request.current_build_number,
        &request.current_build_sha,
        &request.platform,
        &request.arch,
        request.channel,
    )
}

fn latest_available_public_release(
    catalog: &PublicReleaseCatalog,
    current_build_number: u64,
    current_build_sha: &str,
    platform: &str,
    arch: &str,
    channel: UpdateChannel,
) -> Result<Option<PublicReleaseInfo>, UpdateError> {
    let Some(asset_suffix) = public_release_asset_suffix(platform, arch)? else {
        return Ok(None);
    };
    let releases = catalog
        .releases
        .iter()
        .filter(|release| release_matches_channel(release, channel))
        .filter(|release| release.has_download_for(&asset_suffix))
        .collect::<Vec<_>>();
    let latest = releases
        .iter()
        .copied()
        .max_by(compare_public_release_recency);
    let Some(latest) = latest else {
        return Ok(None);
    };
    if latest.matches_build_sha(current_build_sha) {
        return Ok(None);
    }
    if let Some(current_release) = releases
        .iter()
        .copied()
        .filter(|release| release.matches_build_sha(current_build_sha))
        .max_by(compare_public_release_recency)
    {
        return Ok(release_is_newer_than(latest, current_release)
            .then(|| PublicReleaseInfo::from_catalog_release(latest)));
    }
    if latest.build_number <= current_build_number {
        return Ok(None);
    }
    Ok(Some(PublicReleaseInfo::from_catalog_release(latest)))
}

fn release_matches_channel(release: &PublicReleaseCatalogRelease, channel: UpdateChannel) -> bool {
    let Ok(version) = Version::parse(&release.version) else {
        return channel == UpdateChannel::Nightly && release.version == "nightly";
    };
    match channel {
        UpdateChannel::Stable => version.pre.is_empty(),
        UpdateChannel::Rc => version.pre.is_empty() || version.pre.as_str().starts_with("rc."),
        UpdateChannel::Nightly => version.pre.as_str().starts_with("nightly."),
    }
}

fn public_release_asset_suffix(platform: &str, arch: &str) -> Result<Option<String>, UpdateError> {
    if !release_contract::supports_platform_arch(platform, arch)? {
        return Ok(None);
    }
    Ok(Some(format!("{platform}-{arch}.zip")))
}

fn map_http_error(err: ureq::Error) -> UpdateError {
    match err {
        ureq::Error::Status(code, _) => UpdateError::Http(format!("HTTP {code}")),
        ureq::Error::Transport(err) => UpdateError::Http(err.to_string()),
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PublicReleaseCatalog {
    releases: Vec<PublicReleaseCatalogRelease>,
}

#[derive(Clone, Debug, Deserialize)]
struct PublicReleaseCatalogRelease {
    build_id: String,
    build_number: u64,
    version: String,
    released_at: String,
    files: Vec<PublicReleaseCatalogFile>,
}

impl PublicReleaseCatalogRelease {
    fn has_download_for(&self, asset_suffix: &str) -> bool {
        self.files
            .iter()
            .any(|file| file.name.ends_with(asset_suffix))
    }

    fn matches_build_sha(&self, current_build_sha: &str) -> bool {
        let Some(release_sha) = public_release_build_sha(&self.build_id) else {
            return false;
        };
        build_shas_match(current_build_sha, release_sha)
    }
}

impl PublicReleaseInfo {
    fn from_catalog_release(release: &PublicReleaseCatalogRelease) -> Self {
        Self {
            build_id: release.build_id.clone(),
            build_number: release.build_number,
            version: release.version.clone(),
            released_at: release.released_at.clone(),
            download_page_url: PUBLIC_RELEASE_PAGE_URL.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PublicReleaseCatalogFile {
    name: String,
}

fn compare_public_release_recency(
    left: &&PublicReleaseCatalogRelease,
    right: &&PublicReleaseCatalogRelease,
) -> std::cmp::Ordering {
    left.released_at
        .cmp(&right.released_at)
        .then_with(|| left.build_number.cmp(&right.build_number))
}

fn release_is_newer_than(
    candidate: &PublicReleaseCatalogRelease,
    current: &PublicReleaseCatalogRelease,
) -> bool {
    compare_public_release_recency(&candidate, &current).is_gt()
}

fn public_release_build_sha(build_id: &str) -> Option<&str> {
    let suffix = build_id.rsplit('-').next()?;
    is_hex_sha_fragment(suffix).then_some(suffix)
}

fn build_shas_match(current: &str, release: &str) -> bool {
    let current = current.trim();
    if !is_hex_sha_fragment(current) {
        return false;
    }
    let current = current.to_ascii_lowercase();
    let release = release.to_ascii_lowercase();
    current.starts_with(&release) || release.starts_with(&current)
}

fn is_hex_sha_fragment(value: &str) -> bool {
    value.len() >= 7 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_catalog_selects_newer_matching_platform_release() {
        let catalog = catalog(&[
            release(241, "wavecrate-nightly-b241-windows-x86_64.zip"),
            release(242, "wavecrate-nightly-b242-macos-aarch64.zip"),
            release(243, "wavecrate-nightly-b243-macos-x86_64.zip"),
        ]);

        let release = latest_available_public_release(
            &catalog,
            241,
            "unknown",
            "macos",
            "aarch64",
            UpdateChannel::Nightly,
        )
        .expect("release contract")
        .expect("new macos arm release");

        assert_eq!(release.build_number, 242);
        assert_eq!(release.build_id, "wavecrate-nightly-b242");
    }

    #[test]
    fn public_catalog_ignores_current_and_older_releases() {
        let catalog = catalog(&[release(241, "wavecrate-nightly-b241-macos-aarch64.zip")]);

        assert!(
            latest_available_public_release(
                &catalog,
                241,
                "unknown",
                "macos",
                "aarch64",
                UpdateChannel::Nightly
            )
            .expect("release contract")
            .is_none()
        );
    }

    #[test]
    fn public_catalog_ignores_releases_without_platform_download() {
        let catalog = catalog(&[release(242, "wavecrate-nightly-b242-windows-x86_64.zip")]);

        assert!(
            latest_available_public_release(
                &catalog,
                241,
                "unknown",
                "macos",
                "aarch64",
                UpdateChannel::Nightly
            )
            .expect("release contract")
            .is_none()
        );
    }

    #[test]
    fn public_catalog_ignores_historical_linux_entries_for_current_downloads() {
        let catalog = catalog(&[
            release(242, "wavecrate-nightly-b242-macos-aarch64.zip"),
            release(999, "wavecrate-nightly-b999-linux-x86_64.zip"),
        ]);

        let release = latest_available_public_release(
            &catalog,
            241,
            "unknown",
            "macos",
            "aarch64",
            UpdateChannel::Nightly,
        )
        .expect("release contract")
        .expect("new macos arm release");

        assert_eq!(release.build_number, 242);
        assert_eq!(release.build_id, "wavecrate-nightly-b242");
    }

    #[test]
    fn public_catalog_rejects_linux_runtime_platform_even_with_matching_catalog_file() {
        let catalog = catalog(&[release(999, "wavecrate-nightly-b999-linux-x86_64.zip")]);

        assert!(
            latest_available_public_release(
                &catalog,
                241,
                "unknown",
                "linux",
                "x86_64",
                UpdateChannel::Nightly
            )
            .expect("release contract")
            .is_none()
        );
    }

    #[test]
    fn public_release_asset_suffix_accepts_supported_download_targets() {
        assert_eq!(
            public_release_asset_suffix("windows", "x86_64")
                .unwrap()
                .as_deref(),
            Some("windows-x86_64.zip")
        );
        assert_eq!(
            public_release_asset_suffix("macos", "x86_64")
                .unwrap()
                .as_deref(),
            Some("macos-x86_64.zip")
        );
        assert_eq!(
            public_release_asset_suffix("macos", "aarch64")
                .unwrap()
                .as_deref(),
            Some("macos-aarch64.zip")
        );
    }

    #[test]
    fn public_release_asset_suffix_rejects_unsupported_download_targets() {
        assert_eq!(
            public_release_asset_suffix("freebsd", "x86_64").unwrap(),
            None
        );
        assert_eq!(
            public_release_asset_suffix("linux", "x86_64").unwrap(),
            None
        );
        assert_eq!(
            public_release_asset_suffix("windows", "aarch64").unwrap(),
            None
        );
        assert_eq!(
            public_release_asset_suffix("macos", "riscv64").unwrap(),
            None
        );
    }

    #[test]
    fn public_catalog_uses_release_sha_when_build_counters_differ() {
        let catalog = catalog(&[
            release_with_id(
                241,
                "wavecrate-nightly-b241-d7fd3205",
                "wavecrate-nightly-b241-macos-aarch64.zip",
                "2026-06-25T09:22:50.000Z",
            ),
            release_with_id(
                242,
                "wavecrate-nightly-b242-4c969d95",
                "wavecrate-nightly-b242-macos-aarch64.zip",
                "2026-06-25T20:13:25.000Z",
            ),
        ]);

        let release = latest_available_public_release(
            &catalog,
            5_975,
            "d7fd3205abcdef00",
            "macos",
            "aarch64",
            UpdateChannel::Nightly,
        )
        .expect("release contract")
        .expect("newer release by public catalog recency");

        assert_eq!(release.build_id, "wavecrate-nightly-b242-4c969d95");
    }

    #[test]
    fn public_catalog_treats_matching_latest_sha_as_up_to_date() {
        let catalog = catalog(&[
            release_with_id(
                241,
                "wavecrate-nightly-b241-d7fd3205",
                "wavecrate-nightly-b241-macos-aarch64.zip",
                "2026-06-25T09:22:50.000Z",
            ),
            release_with_id(
                242,
                "wavecrate-nightly-b242-4c969d95",
                "wavecrate-nightly-b242-macos-aarch64.zip",
                "2026-06-25T20:13:25.000Z",
            ),
        ]);

        assert!(
            latest_available_public_release(
                &catalog,
                5_975,
                "4c969d95abcdef00",
                "macos",
                "aarch64",
                UpdateChannel::Nightly
            )
            .expect("release contract")
            .is_none()
        );
    }

    #[test]
    fn public_catalog_falls_back_to_build_number_for_unknown_sha() {
        let catalog = catalog(&[release_with_id(
            242,
            "wavecrate-nightly-b242-4c969d95",
            "wavecrate-nightly-b242-macos-aarch64.zip",
            "2026-06-25T20:13:25.000Z",
        )]);

        assert!(
            latest_available_public_release(
                &catalog,
                5_975,
                "<unknown>",
                "macos",
                "aarch64",
                UpdateChannel::Nightly
            )
            .expect("release contract")
            .is_none()
        );
        assert!(
            latest_available_public_release(
                &catalog,
                241,
                "<unknown>",
                "macos",
                "aarch64",
                UpdateChannel::Nightly
            )
            .expect("release contract")
            .is_some()
        );
    }

    #[test]
    fn public_catalog_filters_stable_rc_and_nightly_channels() {
        let catalog = catalog(&[
            release_with_version(
                250,
                "wavecrate-19.1.0-nightly.20260701+abcdef0",
                "19.1.0-nightly.20260701+abcdef0",
                "wavecrate-19.1.0-nightly.20260701+abcdef0-windows-x86_64.zip",
                "2026-07-01T20:00:00.000Z",
            ),
            release_with_version(
                251,
                "wavecrate-19.1.0-rc.1",
                "19.1.0-rc.1",
                "wavecrate-19.1.0-rc.1-windows-x86_64.zip",
                "2026-07-02T20:00:00.000Z",
            ),
            release_with_version(
                252,
                "wavecrate-19.1.0",
                "19.1.0",
                "wavecrate-19.1.0-windows-x86_64.zip",
                "2026-07-03T20:00:00.000Z",
            ),
        ]);

        let stable = latest_available_public_release(
            &catalog,
            1,
            "unknown",
            "windows",
            "x86_64",
            UpdateChannel::Stable,
        )
        .expect("release contract")
        .expect("stable");
        let rc = latest_available_public_release(
            &catalog,
            1,
            "unknown",
            "windows",
            "x86_64",
            UpdateChannel::Rc,
        )
        .expect("release contract")
        .expect("rc");
        let nightly = latest_available_public_release(
            &catalog,
            1,
            "unknown",
            "windows",
            "x86_64",
            UpdateChannel::Nightly,
        )
        .expect("release contract")
        .expect("nightly");

        assert_eq!(stable.version, "19.1.0");
        assert_eq!(rc.version, "19.1.0");
        assert_eq!(nightly.version, "19.1.0-nightly.20260701+abcdef0");
    }

    fn catalog(releases: &[PublicReleaseCatalogRelease]) -> PublicReleaseCatalog {
        PublicReleaseCatalog {
            releases: releases.to_vec(),
        }
    }

    fn release(build_number: u64, file_name: &str) -> PublicReleaseCatalogRelease {
        release_with_id(
            build_number,
            &format!("wavecrate-nightly-b{build_number}"),
            file_name,
            "2026-06-25T20:13:25.000Z",
        )
    }

    fn release_with_id(
        build_number: u64,
        build_id: &str,
        file_name: &str,
        released_at: &str,
    ) -> PublicReleaseCatalogRelease {
        PublicReleaseCatalogRelease {
            build_id: build_id.to_string(),
            build_number,
            version: "nightly".to_string(),
            released_at: released_at.to_string(),
            files: vec![PublicReleaseCatalogFile {
                name: file_name.to_string(),
            }],
        }
    }

    fn release_with_version(
        build_number: u64,
        build_id: &str,
        version: &str,
        file_name: &str,
        released_at: &str,
    ) -> PublicReleaseCatalogRelease {
        PublicReleaseCatalogRelease {
            build_id: build_id.to_string(),
            build_number,
            version: version.to_string(),
            released_at: released_at.to_string(),
            files: vec![PublicReleaseCatalogFile {
                name: file_name.to_string(),
            }],
        }
    }
}
