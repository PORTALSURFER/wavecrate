use crate::sample_sources::SourceId;
use std::path::PathBuf;

/// UI-facing snapshot of the active compare-anchor sample, if any.
///
/// The compare anchor is a single global reference sample used by the compare
/// playback workflow. It stores a stable source/path identity so replay can
/// resolve the anchor even after the user browses elsewhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompareAnchorState {
    /// Source that owns the compare-anchor sample.
    pub source_id: SourceId,
    /// Relative path of the compare-anchor sample inside its source.
    pub relative_path: PathBuf,
    /// User-facing label rendered by compare playback affordances.
    pub label: String,
}
