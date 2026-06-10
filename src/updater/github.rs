mod asset_validation;
mod http_retry;
mod release_selection;
mod release_summary;

use serde::Deserialize;

use super::{RuntimeIdentity, UpdateChannel, UpdateError};

pub use release_summary::ReleaseSummary;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ReleaseAsset {
    pub(super) name: String,
    pub(super) browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Release {
    pub(super) tag_name: String,
    pub(super) prerelease: bool,
    pub(super) html_url: String,
    pub(super) published_at: Option<String>,
    pub(super) assets: Vec<ReleaseAsset>,
}

pub(super) fn fetch_release_with_assets(
    repo: &str,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
) -> Result<Release, UpdateError> {
    let releases = http_retry::fetch_releases(repo)?;
    release_selection::select_release_with_assets(releases, channel, identity)
}

pub(super) fn fetch_release_by_tag_with_assets(
    repo: &str,
    tag: &str,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
) -> Result<Release, UpdateError> {
    let release = http_retry::fetch_release_by_tag(repo, tag)?;
    asset_validation::validate_tagged_release_assets(&release, tag, channel, identity)?;
    Ok(release)
}

pub(super) fn list_releases_with_assets(
    repo: &str,
    channel: UpdateChannel,
    identity: &RuntimeIdentity,
    limit: usize,
) -> Result<Vec<ReleaseSummary>, UpdateError> {
    let releases = http_retry::fetch_releases(repo)?;
    release_selection::list_releases_with_assets(releases, channel, identity, limit)
}

pub(super) fn find_asset<'a>(release: &'a Release, name: &str) -> Option<&'a ReleaseAsset> {
    asset_validation::find_asset(release, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_shape() {
        let json = r#"
        {
          "tag_name": "v0.1.0",
          "prerelease": false,
          "html_url": "https://example.invalid/release",
          "published_at": "2025-01-01T00:00:00Z",
          "assets": [
            { "name": "foo.zip", "browser_download_url": "https://example.invalid/foo.zip" }
          ]
        }"#;
        let parsed: Release = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.tag_name, "v0.1.0");
        assert!(!parsed.prerelease);
        assert_eq!(parsed.assets.len(), 1);
        assert_eq!(parsed.assets[0].name, "foo.zip");
    }
}
