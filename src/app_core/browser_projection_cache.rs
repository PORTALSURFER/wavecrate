//! Browser projection cache contracts owned by app-core.

use crate::app_core::state::PlaybackAgeBucket;
use crate::sample_sources::SourceId;
use std::path::PathBuf;

/// Retained browser-row projection fields keyed by absolute entry index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedBrowserRowCacheEntry {
    /// Stable row-identity hash derived from the live entry relative path.
    pub row_identity_hash: u64,
    /// Relative sample path used for metadata preloads and label fallback.
    pub relative_path: PathBuf,
    /// Stable rendered row label for the browser list.
    pub row_label: String,
    /// Triage column index (`0..=2`) for this row.
    pub column_index: usize,
    /// Signed keep/trash rating level for this row (`-3..=3`).
    pub rating_level: i8,
    /// Playback-age bucket projected for row-aging visuals.
    pub playback_age_bucket: PlaybackAgeBucket,
    /// Stable rendered inline metadata label for the browser list row.
    pub bucket_label: String,
    /// Whether the backing sample file is currently marked missing.
    pub missing: bool,
    /// Whether the backing sample is marked looped.
    pub looped: bool,
    /// Whether the backing sample is marked as a confirmed keep lock.
    pub locked: bool,
    /// Cached BPM bits used to detect metadata changes without rebuilding label text.
    pub bpm_value_bits: Option<u32>,
    /// Whether the backing sample currently carries the long-sample marker.
    pub long_sample_mark: bool,
    /// Monotonic usage tick used for bounded least-recently-used eviction.
    pub last_used_tick: u64,
}

/// Visible browser window metadata retained for incremental BPM preloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedBrowserPreloadWindow {
    /// Selected source associated with the last preload window.
    pub source_id: Option<SourceId>,
    /// Visible-row revision associated with the last preload window.
    pub visible_rows_revision: u64,
    /// First visible row index covered by the last preload window.
    pub window_start: usize,
    /// Number of rows covered by the last preload window.
    pub window_len: usize,
}

/// Retained selected-row lookup representation for browser projections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProjectedSelectedPathsLookup {
    /// Fast path for the common single-selection case.
    Single(usize),
    /// Dense lookup used for larger multi-selections.
    Dense(Vec<bool>),
}

/// UI row state for one requested path in an active auto-rename batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AutoRenameBatchRowState {
    Queued,
    Active,
    Completed,
    Skipped,
    Failed,
}
