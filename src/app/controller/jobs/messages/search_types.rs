//! Search lane DTOs for async browser row queries.

use super::*;

/// Background browser search request captured from current controller state.
#[derive(Debug)]
pub(crate) struct SearchJob {
    /// Monotonic request identifier used to discard stale async search results.
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) source_root: PathBuf,
    pub(crate) query: String,
    pub(crate) filter: crate::app::state::TriageFlagFilter,
    /// Rating levels selected for filtering (`-3..=3`, plus `4` for locked keeps).
    pub(crate) rating_filter: BTreeSet<i8>,
    /// Playback-age chips selected for filtering older or never-played samples.
    pub(crate) playback_age_filter: BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
    /// Whether the result set should keep only session-marked samples.
    pub(crate) marked_only: bool,
    /// Filter for samples known to have tag-derived filenames.
    pub(crate) tag_named_filter: crate::app::state::TagNamedFilter,
    /// Sidebar metadata facet filters selected for the browser.
    pub(crate) sidebar_filters: crate::app::state::BrowserSidebarFilterState,
    /// BPM metadata aligned by relative path for sidebar BPM facets.
    pub(crate) sidebar_bpm_values: BTreeMap<PathBuf, Option<f32>>,
    /// Session-marked sample paths for the active source.
    pub(crate) marked_paths: BTreeSet<PathBuf>,
    pub(crate) sort: crate::app::state::SampleBrowserSort,
    pub(crate) similar_query: Option<crate::app::state::SimilarQuery>,
    pub(crate) duplicate_cleanup: Option<crate::app::state::BrowserDuplicateCleanupState>,
    pub(crate) folder_selection: Option<BTreeSet<PathBuf>>,
    pub(crate) folder_negated: Option<BTreeSet<PathBuf>>,
    pub(crate) file_scope_mode: crate::app::state::FolderFileScopeMode,
    /// Metadata-only changed paths that can be refreshed in place when path order is unchanged.
    pub(crate) metadata_delta_paths: Vec<PathBuf>,
    /// Reference timestamp used to classify playback-age buckets consistently within one job.
    pub(crate) playback_age_now_unix_secs: i64,
}

/// Async search results aligned to the queued browser snapshot.
#[derive(Debug)]
pub(crate) struct SearchResult {
    /// Request identifier echoed from [`SearchJob::request_id`].
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) query: String,
    pub(crate) visible: crate::app::state::VisibleRows,
    /// Shared triage row indexes tagged as trash.
    pub(crate) trash: Arc<[usize]>,
    /// Shared triage row indexes tagged as neutral.
    pub(crate) neutral: Arc<[usize]>,
    /// Shared triage row indexes tagged as keep.
    pub(crate) keep: Arc<[usize]>,
    /// Shared query score payload aligned to absolute row indexes.
    pub(crate) scores: Arc<[Option<i64>]>,
}
