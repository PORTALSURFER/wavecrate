//! Public PortalSurfer release-catalog checks.

use serde::Deserialize;

use crate::http_client;

use super::UpdateError;

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
}

impl PublicReleaseCheckRequest {
    /// Build a request for the current binary and target.
    pub fn current(current_build_number: u64) -> Self {
        Self {
            catalog_url: PUBLIC_RELEASE_CATALOG_URL.to_string(),
            current_build_number,
            current_build_sha: env!("WAVECRATE_BUILD_GIT_SHA").to_string(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
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
    Ok(latest_available_public_release(
        &catalog,
        request.current_build_number,
        &request.current_build_sha,
        &request.platform,
        &request.arch,
    ))
}

fn latest_available_public_release(
    catalog: &PublicReleaseCatalog,
    current_build_number: u64,
    current_build_sha: &str,
    platform: &str,
    arch: &str,
) -> Option<PublicReleaseInfo> {
    let asset_suffix = public_release_asset_suffix(platform, arch)?;
    let releases = catalog
        .releases
        .iter()
        .filter(|release| release.has_download_for(&asset_suffix))
        .collect::<Vec<_>>();
    let latest = releases
        .iter()
        .copied()
        .max_by(compare_public_release_recency)?;
    if latest.matches_build_sha(current_build_sha) {
        return None;
    }
    if let Some(current_release) = releases
        .iter()
        .copied()
        .filter(|release| release.matches_build_sha(current_build_sha))
        .max_by(compare_public_release_recency)
    {
        return release_is_newer_than(latest, current_release)
            .then(|| PublicReleaseInfo::from_catalog_release(latest));
    }
    if latest.build_number <= current_build_number {
        return None;
    }
    Some(PublicReleaseInfo::from_catalog_release(latest))
}

fn public_release_asset_suffix(platform: &str, arch: &str) -> Option<String> {
    let platform = match platform {
        "macos" | "windows" => platform,
        _ => return None,
    };
    let arch = match arch {
        "aarch64" | "x86_64" => arch,
        _ => return None,
    };
    Some(format!("{platform}-{arch}.zip"))
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

        let release = latest_available_public_release(&catalog, 241, "unknown", "macos", "aarch64")
            .expect("new macos arm release");

        assert_eq!(release.build_number, 242);
        assert_eq!(release.build_id, "wavecrate-nightly-b242");
    }

    #[test]
    fn public_catalog_ignores_current_and_older_releases() {
        let catalog = catalog(&[release(241, "wavecrate-nightly-b241-macos-aarch64.zip")]);

        assert!(
            latest_available_public_release(&catalog, 241, "unknown", "macos", "aarch64").is_none()
        );
    }

    #[test]
    fn public_catalog_ignores_releases_without_platform_download() {
        let catalog = catalog(&[release(242, "wavecrate-nightly-b242-windows-x86_64.zip")]);

        assert!(
            latest_available_public_release(&catalog, 241, "unknown", "macos", "aarch64").is_none()
        );
    }

    #[test]
    fn public_release_asset_suffix_rejects_unknown_targets() {
        assert!(public_release_asset_suffix("freebsd", "x86_64").is_none());
        assert!(public_release_asset_suffix("linux", "x86_64").is_none());
        assert!(public_release_asset_suffix("macos", "riscv64").is_none());
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
        )
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
                "aarch64"
            )
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
            latest_available_public_release(&catalog, 5_975, "<unknown>", "macos", "aarch64")
                .is_none()
        );
        assert!(
            latest_available_public_release(&catalog, 241, "<unknown>", "macos", "aarch64")
                .is_some()
        );
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
}
