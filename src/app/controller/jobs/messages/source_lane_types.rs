//! Source hydration, folder projection, and related source-lane DTOs.

use super::*;
use crate::app::controller::LoadEntriesError;
use crate::app::controller::library::source_folders::{FolderProjectionView, FolderTreeSnapshot};
use crate::sample_sources::WavEntry;

/// Controller-owned source hydration lanes used to load source snapshots off the UI thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceHydrationKind {
    /// Hydrate the currently selected source that drives browser and waveform state.
    ActiveSelection,
    /// Hydrate a source assigned to the inactive retained folder pane.
    InactivePane,
}

/// Background source-hydration request that prepares one source snapshot for a pane.
#[derive(Debug)]
pub(crate) struct SourceHydrationJob {
    /// Monotonic request identifier used to discard stale results.
    pub(crate) request_id: u64,
    /// Sidebar pane that owns the source assignment.
    pub(crate) pane: FolderPaneId,
    /// Logical hydration lane for result application.
    pub(crate) kind: SourceHydrationKind,
    /// Source identifier that should be hydrated.
    pub(crate) source_id: SourceId,
    /// Source root used to open the DB and derive folder availability.
    pub(crate) source_root: PathBuf,
    /// Target page size for page-0 entry loading.
    pub(crate) page_size: usize,
    /// Optional cached page-0 entries cloned at dispatch time.
    pub(crate) cached_page: Option<Vec<WavEntry>>,
    /// Total entry count associated with `cached_page`, when present.
    pub(crate) cached_total: Option<usize>,
    /// Page size associated with `cached_page`, when present.
    pub(crate) cached_page_size: Option<usize>,
    /// Whether startup should stop after the minimum page-0 payload and defer follow-up work.
    pub(crate) defer_startup_follow_up_work: bool,
}

/// Compact source snapshot prepared off the UI thread for later controller apply.
#[derive(Debug)]
pub(crate) struct SourceHydrationSnapshot {
    /// Page-0 wav entries for the hydrated source.
    pub(crate) entries: Vec<WavEntry>,
    /// Total number of wav entries in the source.
    pub(crate) total: usize,
    /// Page size used to interpret `entries`.
    pub(crate) page_size: usize,
    /// Prebuilt normalized path lookup aligned with the hydrated page.
    pub(crate) path_lookup: HashMap<PathBuf, usize>,
    /// Folder availability derived from the hydrated page.
    pub(crate) available_folders: BTreeSet<PathBuf>,
    /// Immutable folder-tree snapshot derived from `available_folders`.
    pub(crate) folder_tree: FolderTreeSnapshot,
    /// Browser feature metadata aligned to `entries` for first-paint row badges.
    pub(crate) feature_cache: Option<crate::app::controller::FeatureCache>,
    /// Whether the snapshot reused the page-0 wav cache instead of querying the DB.
    pub(crate) from_cache: bool,
    /// Whether folder projection and feature metadata still need a deferred follow-up pass.
    pub(crate) deferred_follow_up_work: bool,
}

/// Result of one async browser feature-cache refresh.
#[derive(Debug)]
pub(crate) struct BrowserFeatureCacheRefreshResult {
    /// Request identifier used to drop stale refresh results.
    pub(crate) request_id: u64,
    /// Source whose browser feature metadata was refreshed.
    pub(crate) source_id: SourceId,
    /// Snapshot key the refreshed rows were built against.
    pub(crate) key: crate::app::controller::FeatureCacheKey,
    /// Refreshed cache payload or the terminal load error.
    pub(crate) result: Result<crate::app::controller::FeatureCache, String>,
}

/// Background folder projection request for one pane-scoped folder browser.
#[derive(Debug)]
pub(crate) struct FolderProjectionJob {
    /// Monotonic request identifier used to discard stale results.
    pub(crate) request_id: u64,
    /// Sidebar pane whose folder browser rows are being projected.
    pub(crate) pane: FolderPaneId,
    /// Source identifier that owns the folder browser state.
    pub(crate) source_id: SourceId,
    /// Immutable retained folder model snapshot captured on the controller thread.
    pub(crate) model: crate::app::controller::library::source_folders::FolderBrowserModel,
    /// Projection workload to execute off the UI thread.
    pub(crate) work: FolderProjectionWork,
    /// Whether a source is assigned to the pane during projection.
    pub(crate) has_source: bool,
}

/// Off-thread folder projection workload kinds.
#[derive(Debug)]
pub(crate) enum FolderProjectionWork {
    /// Rebuild available folders, reconcile the retained model, and project rows.
    RefreshAvailable {
        /// Source root used to validate folder paths against disk.
        source_root: PathBuf,
        /// Currently loaded relative wav paths used to derive folder availability.
        loaded_relative_paths: Vec<PathBuf>,
        /// Cached disk folders retained for the source.
        disk_folders: BTreeSet<PathBuf>,
        /// Cached available folders used for empty-entry reuse semantics.
        cached_available: BTreeSet<PathBuf>,
        /// Visibility mode associated with `cached_available`.
        cached_available_show_all_folders: bool,
        /// Whether a waveform load for this source is still pending.
        pending_wav_load: bool,
    },
    /// Reproject rows from an existing immutable tree snapshot.
    Reproject {
        /// Immutable tree snapshot reused across row projections.
        snapshot: FolderTreeSnapshot,
    },
}

/// Result payload for one completed folder projection request.
#[derive(Debug)]
pub(crate) struct FolderProjectionSnapshot {
    /// Reconciled folder-browser model snapshot that should replace the retained cache entry.
    pub(crate) model: crate::app::controller::library::source_folders::FolderBrowserModel,
    /// Immutable tree snapshot aligned to `model.available`.
    pub(crate) tree: FolderTreeSnapshot,
    /// Projected folder-browser view fields ready for UI apply.
    pub(crate) view: FolderProjectionView,
}

/// Result of one background folder projection request.
#[derive(Debug)]
pub(crate) struct FolderProjectionResult {
    /// Request identifier echoed from [`FolderProjectionJob::request_id`].
    pub(crate) request_id: u64,
    /// Sidebar pane whose folder browser rows are being projected.
    pub(crate) pane: FolderPaneId,
    /// Source identifier that owns the folder browser state.
    pub(crate) source_id: SourceId,
    /// Worker time spent hydrating or projecting the folder rows.
    pub(crate) elapsed: Duration,
    /// Folder projection payload prepared off the UI thread.
    pub(crate) snapshot: FolderProjectionSnapshot,
}

/// Result of one background source hydration request.
#[derive(Debug)]
pub(crate) struct SourceHydrationResult {
    /// Request identifier echoed from [`SourceHydrationJob::request_id`].
    pub(crate) request_id: u64,
    /// Sidebar pane that owns the source assignment.
    pub(crate) pane: FolderPaneId,
    /// Logical hydration lane for result application.
    pub(crate) kind: SourceHydrationKind,
    /// Source identifier associated with this hydration attempt.
    pub(crate) source_id: SourceId,
    /// Worker time spent loading/building the snapshot.
    pub(crate) elapsed: Duration,
    /// Hydrated snapshot or the terminal load error.
    pub(crate) result: Result<SourceHydrationSnapshot, LoadEntriesError>,
}

/// Result of a background folder scan for a source root.
#[derive(Debug)]
pub(crate) struct FolderScanResult {
    /// Request identifier for this scan.
    pub(crate) request_id: u64,
    /// Source identifier associated with the scan.
    pub(crate) source_id: SourceId,
    /// Relative folder paths discovered on disk.
    pub(crate) folders: BTreeSet<PathBuf>,
}
