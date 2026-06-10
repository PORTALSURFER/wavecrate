use super::Release;
use super::asset_validation;
use super::release_summary::ReleaseSummary;
use crate::updater::{RuntimeIdentity, UpdateChannel, UpdateError};

pub(super) fn list_releases_with_assets(
    releases: Vec<Release>,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
    limit: usize,
) -> Result<Vec<ReleaseSummary>, UpdateError> {
    let mut matches = Vec::new();
    for release in releases {
        if release_has_required_assets(&release, channel, identity)? {
            matches.push(ReleaseSummary::from_release(release));
        }
        if matches.len() >= limit {
            break;
        }
    }
    Ok(matches)
}

pub(super) fn select_release_with_assets(
    releases: Vec<Release>,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
) -> Result<Release, UpdateError> {
    for release in releases {
        if release_has_required_assets(&release, channel, identity)? {
            return Ok(release);
        }
    }
    Err(UpdateError::Invalid(format!(
        "No {channel:?} release with required assets found"
    )))
}

fn release_has_required_assets(
    release: &Release,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
) -> Result<bool, UpdateError> {
    match channel {
        UpdateChannel::Stable => {
            if release.prerelease {
                return Ok(false);
            }
            let Some(version_text) = release.tag_name.strip_prefix('v') else {
                return Ok(false);
            };
            asset_validation::stable_release_has_assets(release, version_text, identity)
        }
        UpdateChannel::Nightly => {
            if release.tag_name != "nightly" {
                return Ok(false);
            }
            asset_validation::nightly_release_has_assets(release, identity)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ReleaseAsset;
    use super::*;

    #[test]
    fn stable_selection_skips_prereleases_and_releases_missing_assets() {
        let identity = identity(UpdateChannel::Stable);
        let selected = select_release_with_assets(
            vec![
                release("v9.9.9", true, &stable_assets("9.9.9")),
                release(
                    "v1.2.2",
                    false,
                    &["wavecrate-v1.2.2-windows-x86_64.zip".to_string()],
                ),
                release("v1.2.3", false, &stable_assets("1.2.3")),
            ],
            UpdateChannel::Stable,
            &identity,
        )
        .expect("stable release");

        assert_eq!(selected.tag_name, "v1.2.3");
    }

    #[test]
    fn nightly_selection_requires_nightly_tag_and_assets() {
        let identity = identity(UpdateChannel::Nightly);
        let selected = select_release_with_assets(
            vec![
                release("v1.2.3", false, &stable_assets("1.2.3")),
                release(
                    "nightly",
                    true,
                    &["wavecrate-nightly-windows-x86_64.zip".to_string()],
                ),
                release("nightly", true, &nightly_assets()),
            ],
            UpdateChannel::Nightly,
            &identity,
        )
        .expect("nightly release");

        assert_eq!(selected.tag_name, "nightly");
    }

    #[test]
    fn summary_listing_preserves_matching_release_order_and_limit() {
        let identity = identity(UpdateChannel::Stable);
        let summaries = list_releases_with_assets(
            vec![
                release("v1.2.3", false, &stable_assets("1.2.3")),
                release("v1.2.2", false, &stable_assets("1.2.2")),
            ],
            UpdateChannel::Stable,
            &identity,
            1,
        )
        .expect("summaries");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].tag, "v1.2.3");
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

    fn stable_assets(version: &str) -> Vec<String> {
        vec![
            format!("wavecrate-v{version}-windows-x86_64.zip"),
            format!("checksums-v{version}.txt"),
            format!("checksums-v{version}.txt.sig"),
        ]
    }

    fn nightly_assets() -> Vec<String> {
        vec![
            "wavecrate-nightly-windows-x86_64.zip".to_string(),
            "checksums-nightly.txt".to_string(),
            "checksums-nightly.txt.sig".to_string(),
        ]
    }

    fn release(tag: &str, prerelease: bool, assets: &[String]) -> Release {
        Release {
            tag_name: tag.to_string(),
            prerelease,
            html_url: format!("https://example.invalid/{tag}"),
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
