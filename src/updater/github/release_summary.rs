use super::Release;

/// Public-facing release metadata for the updater UI.
#[derive(Debug, Clone)]
pub struct ReleaseSummary {
    /// Git tag name (e.g. `v0.384.0` or `nightly`).
    pub tag: String,
    /// HTML URL for the release page.
    pub html_url: String,
    /// Publication timestamp (RFC3339), if present.
    pub published_at: Option<String>,
}

impl ReleaseSummary {
    pub(super) fn from_release(release: Release) -> Self {
        Self {
            tag: release.tag_name,
            html_url: release.html_url,
            published_at: release.published_at,
        }
    }
}
