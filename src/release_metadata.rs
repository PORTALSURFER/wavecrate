//! Build-time release metadata embedded into Wavecrate binaries.

/// Release channel embedded into the current binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    /// Stable public release.
    Stable,
    /// Release candidate build.
    Rc,
    /// Nightly development build.
    Nightly,
    /// Unknown or locally supplied channel.
    Other,
}

impl ReleaseChannel {
    /// Parse the canonical build-time channel label.
    pub fn parse(value: &str) -> Self {
        match value {
            "stable" => Self::Stable,
            "rc" => Self::Rc,
            "nightly" => Self::Nightly,
            _ => Self::Other,
        }
    }

    /// User-facing channel label.
    pub fn display_label(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::Rc => "Release Candidate",
            Self::Nightly => "Nightly",
            Self::Other => "Custom",
        }
    }
}

/// Release metadata compiled into the binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseMetadata {
    /// Full release version, including prerelease/build metadata when present.
    pub version: &'static str,
    /// Canonical channel string: `stable`, `rc`, or `nightly`.
    pub channel: &'static str,
    /// Git commit SHA or `<unknown>`.
    pub commit: &'static str,
    /// UTC build date in `YYYY-MM-DD` form.
    pub build_date: &'static str,
    /// Stable target version this build belongs to.
    pub target_version: &'static str,
    /// Numeric build counter.
    pub build_number: &'static str,
}

impl ReleaseMetadata {
    /// Parsed release channel.
    pub fn release_channel(self) -> ReleaseChannel {
        ReleaseChannel::parse(self.channel)
    }
}

/// Metadata for the current binary.
pub const CURRENT: ReleaseMetadata = ReleaseMetadata {
    version: env!("WAVECRATE_RELEASE_VERSION"),
    channel: env!("WAVECRATE_RELEASE_CHANNEL"),
    commit: env!("WAVECRATE_BUILD_GIT_SHA"),
    build_date: env!("WAVECRATE_RELEASE_BUILD_DATE"),
    target_version: env!("WAVECRATE_RELEASE_TARGET_VERSION"),
    build_number: env!("WAVECRATE_BUILD_NUMBER"),
};
