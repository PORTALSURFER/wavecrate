//! Job message and DTO types shared across controller workers.

use super::*;
use crate::app::controller::LoadEntriesError;
use crate::app::controller::library::source_folders::{FolderProjectionView, FolderTreeSnapshot};
use crate::app::controller::library::wavs::waveform_rendering::PreparedWaveformVisual;
use crate::sample_sources::WavEntry;
use crate::waveform::{DecodedWaveform, WaveformChannelView, WaveformRenderViewport};

#[derive(Debug)]
pub(crate) enum JobMessage {
    WavLoaded(WavLoadResult),
    SourceHydrated(SourceHydrationResult),
    BrowserFeatureCacheRefreshed(BrowserFeatureCacheRefreshResult),
    FolderProjected(FolderProjectionResult),
    MetadataMutationFinished(MetadataMutationResult),
    ConfigPersistFinished(ConfigPersistResult),
    WaveformRendered(WaveformRenderResult),
    WaveformTransientsComputed(WaveformTransientResult),
    AudioLoaded(AudioLoadResult),
    RecordingWaveformLoaded(RecordingWaveformLoadResult),
    Scan(ScanJobMessage),
    FolderScanFinished(FolderScanResult),
    SourceWatch(SourceWatchEvent),
    TrashMove(trash_move::TrashMoveMessage),
    FolderDeleteRecoveryFinished(DeleteRecoveryReport),
    FileOps(FileOpMessage),
    Analysis(AnalysisJobMessage),
    AnalysisFailuresLoaded(AnalysisFailuresResult),
    FocusedSimilarityLoaded(FocusedSimilarityResult),
    LoadedSimilarityQueryBuilt(LoadedSimilarityQueryResult),
    UmapBuilt(UmapBuildResult),
    UmapClustersBuilt(UmapClusterBuildResult),
    SimilarityPrepared(SimilarityPrepResult),
    UpdateChecked(UpdateCheckResult),
    IssueGatewayCreated(IssueGatewayCreateResult),
    IssueGatewayAuthed(IssueGatewayAuthResult),
    IssueTokenLoaded(IssueTokenLoadResult),
    IssueTokenSaved(IssueTokenSaveResult),
    IssueTokenDeleted(IssueTokenDeleteResult),
    BrowserSearchFinished(SearchResult),
    SourceDbMaintenanceFinished(SourceDbMaintenanceResult),
    SelectionExport(SelectionExportMessage),
    Normalized(NormalizationResult),
}

/// One sample-source metadata mutation that should execute off the UI thread.
#[derive(Clone, Debug)]
pub(crate) enum SourceMetadataMutationOp {
    /// Persist a tag plus keep-lock state for one sample.
    SetTagAndLocked {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// New rating tag to store.
        tag: crate::sample_sources::Rating,
        /// New keep-lock state to store.
        locked: bool,
    },
    /// Persist one loop-marker state change.
    SetLooped {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// New loop-marker state to store.
        looped: bool,
    },
    /// Persist one playback-age timestamp update.
    SetLastPlayedAt {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// New playback timestamp in Unix seconds.
        played_at: i64,
    },
}

/// One analysis-database metadata mutation that should execute off the UI thread.
#[derive(Clone, Debug)]
pub(crate) enum AnalysisMetadataMutationOp {
    /// Persist one BPM value for a sample.
    SetBpm {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// New BPM value, or `None` to clear it.
        bpm: Option<f32>,
    },
    /// Persist loaded-duration metadata for one sample.
    SetLoadedDuration {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Measured waveform duration in seconds.
        duration_seconds: f32,
        /// Sample rate associated with the decoded waveform.
        sample_rate: u32,
        /// Optional long-sample mark aligned with the decoded waveform.
        long_sample_mark: Option<bool>,
    },
}

/// Background metadata mutation request for one source.
#[derive(Clone, Debug)]
pub(crate) struct MetadataMutationJob {
    /// Monotonic request identifier used for completion tracking.
    pub(crate) request_id: u64,
    /// Source that owns every mutation in this batch.
    pub(crate) source_id: SourceId,
    /// Source root used to open source and analysis databases.
    pub(crate) source_root: PathBuf,
    /// Deduped relative sample paths touched by any mutation in the batch.
    pub(crate) paths: BTreeSet<PathBuf>,
    /// Source-db mutations to apply.
    pub(crate) source_ops: Vec<SourceMetadataMutationOp>,
    /// Analysis-db mutations to apply.
    pub(crate) analysis_ops: Vec<AnalysisMetadataMutationOp>,
}

/// Completion payload for one background metadata mutation batch.
#[derive(Debug)]
pub(crate) struct MetadataMutationResult {
    /// Request identifier echoed from the queued job.
    pub(crate) request_id: u64,
    /// Source that owned the batch.
    pub(crate) source_id: SourceId,
    /// Relative sample paths touched by the batch.
    pub(crate) paths: BTreeSet<PathBuf>,
    /// Worker time spent applying metadata writes.
    pub(crate) elapsed: Duration,
    /// Terminal mutation outcome.
    pub(crate) result: Result<(), String>,
}

/// Deferred configuration persistence request that should never block frame prep.
#[derive(Clone, Debug)]
pub(crate) enum ConfigPersistJob {
    /// Persist the current app configuration after a debounced volume change.
    SaveVolume {
        /// Request identifier used for stale-result handling.
        request_id: u64,
        /// Normalized clamped volume value.
        volume: f32,
    },
}

/// Completion payload for one deferred configuration persistence job.
#[derive(Debug)]
pub(crate) struct ConfigPersistResult {
    /// Request identifier echoed from the queued job.
    pub(crate) request_id: u64,
    /// Job lane that produced this result.
    pub(crate) job: ConfigPersistJob,
    /// Worker time spent persisting configuration.
    pub(crate) elapsed: Duration,
    /// Terminal persistence outcome.
    pub(crate) result: Result<(), String>,
}

/// Stable render key for one waveform raster request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaveformRenderKey {
    /// Decode token for the waveform sample content.
    pub(crate) cache_token: u64,
    /// Texture width used for raster generation.
    pub(crate) texture_width: u32,
    /// Viewport height used for raster generation.
    pub(crate) height: u32,
    /// Channel-view mode used by raster generation.
    pub(crate) channel_view: WaveformChannelView,
    /// Bitwise normalized view start used for reuse/staleness checks.
    pub(crate) view_start_bits: u64,
    /// Bitwise normalized view end used for reuse/staleness checks.
    pub(crate) view_end_bits: u64,
    /// Optional transient-visual token used by marker overlays.
    pub(crate) transient_visual_token: Option<u64>,
}

/// Background waveform raster request.
#[derive(Clone)]
pub(crate) struct WaveformRenderJob {
    /// Monotonic request identifier used to drop stale results.
    pub(crate) request_id: u64,
    /// Stable render key that describes the requested raster.
    pub(crate) key: WaveformRenderKey,
    /// Immutable decoded waveform payload used by the renderer.
    pub(crate) decoded: Arc<DecodedWaveform>,
    /// Renderer clone used to produce the raster.
    pub(crate) renderer: crate::waveform::WaveformRenderer,
    /// Channel-view mode used by raster generation.
    pub(crate) channel_view: WaveformChannelView,
    /// Render viewport used for the raster request.
    pub(crate) viewport: WaveformRenderViewport,
    /// Optional transient overlay input aligned with `decoded`.
    pub(crate) transients: Option<Arc<[f32]>>,
}

/// Completion payload for one waveform render request.
#[derive(Debug)]
pub(crate) struct WaveformRenderResult {
    /// Request identifier echoed from the queued job.
    pub(crate) request_id: u64,
    /// Stable render key that should still match on apply.
    pub(crate) key: WaveformRenderKey,
    /// Worker time spent rasterizing the waveform image.
    pub(crate) elapsed: Duration,
    /// Raster result or terminal render error.
    pub(crate) result: Result<PreparedWaveformVisual, String>,
}

/// Completion payload for one deferred waveform transient-marker computation.
#[derive(Debug)]
pub(crate) struct WaveformTransientResult {
    /// Request identifier echoed from the queued job.
    pub(crate) request_id: u64,
    /// Decode cache token that still must match on apply.
    pub(crate) cache_token: u64,
    /// Worker time spent computing transient markers.
    pub(crate) elapsed: Duration,
    /// Transient markers or the terminal compute error.
    pub(crate) result: Result<Arc<[f32]>, String>,
}

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
    /// Session-marked sample paths for the active source.
    pub(crate) marked_paths: BTreeSet<PathBuf>,
    pub(crate) sort: crate::app::state::SampleBrowserSort,
    pub(crate) similar_query: Option<crate::app::state::SimilarQuery>,
    pub(crate) duplicate_cleanup: Option<crate::app::state::BrowserDuplicateCleanupState>,
    pub(crate) folder_selection: Option<BTreeSet<PathBuf>>,
    pub(crate) folder_negated: Option<BTreeSet<PathBuf>>,
    pub(crate) file_scope_mode: crate::app::state::FolderFileScopeMode,
    /// Reference timestamp used to classify playback-age buckets consistently within one job.
    pub(crate) playback_age_now_unix_secs: i64,
}

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

/// Request payload for creating a new issue through the gateway worker.
#[derive(Debug)]
pub(crate) struct IssueGatewayJob {
    /// Bearer token used to authorize issue creation.
    pub(crate) token: String,
    /// Serialized issue request body sent to the gateway API.
    pub(crate) request: crate::issue_gateway::api::CreateIssueRequest,
}

/// Poll request for gateway-issued auth completion by request id.
#[derive(Debug)]
pub(crate) struct IssueGatewayPollJob {
    /// Opaque request identifier returned by the create/auth kickoff flow.
    pub(crate) request_id: String,
}

/// Outcome of an issue-create request sent through the gateway worker.
#[derive(Debug)]
pub(crate) struct IssueGatewayCreateResult {
    /// API result payload or domain error returned by the gateway client.
    pub(crate) result: Result<
        crate::issue_gateway::api::CreateIssueResponse,
        crate::issue_gateway::api::CreateIssueError,
    >,
}

/// Outcome of polling the gateway for an authenticated issue token.
#[derive(Debug)]
pub(crate) struct IssueGatewayAuthResult {
    /// Auth token when polling succeeds, or the terminal polling error.
    pub(crate) result: Result<String, crate::issue_gateway::api::IssueAuthError>,
}

/// Request to save a GitHub issue token to persistent storage.
#[derive(Debug)]
pub(crate) struct IssueTokenSaveJob {
    /// Token value to persist.
    pub(crate) token: String,
    /// Whether the token modal should reopen after save completion.
    pub(crate) reopen_modal: bool,
}

/// Result from attempting to load a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenLoadResult {
    pub(crate) result: Result<Option<String>, crate::issue_gateway::IssueTokenStoreError>,
}

/// Result from attempting to save a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenSaveResult {
    pub(crate) token: String,
    pub(crate) reopen_modal: bool,
    pub(crate) result: Result<(), crate::issue_gateway::IssueTokenStoreError>,
}

/// Result from attempting to delete a GitHub issue token.
#[derive(Debug)]
pub(crate) struct IssueTokenDeleteResult {
    pub(crate) result: Result<(), crate::issue_gateway::IssueTokenStoreError>,
}

#[derive(Debug, Clone)]
pub(crate) struct UmapBuildJob {
    pub(crate) model_id: String,
    pub(crate) umap_version: String,
    pub(crate) source_id: SourceId,
}

#[derive(Debug)]
pub(crate) struct UmapBuildResult {
    pub(crate) umap_version: String,
    pub(crate) result: Result<(), String>,
}

#[derive(Debug, Clone)]
pub(crate) struct UmapClusterBuildJob {
    pub(crate) model_id: String,
    pub(crate) umap_version: String,
    pub(crate) source_id: Option<SourceId>,
}

#[derive(Debug)]
pub(crate) struct UmapClusterBuildResult {
    pub(crate) source_id: Option<SourceId>,
    pub(crate) result: Result<crate::analysis::hdbscan::HdbscanStats, String>,
}

#[derive(Debug)]
pub(crate) struct SimilarityPrepOutcome {
    pub(crate) cluster_stats: crate::analysis::hdbscan::HdbscanStats,
}

#[derive(Debug)]
pub(crate) struct SimilarityPrepResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<SimilarityPrepOutcome, String>,
}

/// Path-based similarity highlight payload computed off the controller thread.
#[derive(Debug)]
pub(crate) struct FocusedSimilarityPaths {
    /// Stable sample identifier for the focused anchor sample.
    pub(crate) sample_id: String,
    /// Candidate relative paths in descending similarity order.
    pub(crate) paths: Vec<PathBuf>,
    /// Similarity scores aligned to [`Self::paths`].
    pub(crate) scores: Vec<f32>,
    /// Focused entry index captured when the request was queued.
    pub(crate) anchor_index: Option<usize>,
}

/// Async result for one focused-similarity highlight refresh request.
#[derive(Debug)]
pub(crate) struct FocusedSimilarityResult {
    /// Monotonic request identifier used to discard stale async results.
    pub(crate) request_id: u64,
    /// Source that owned the focused selection when the request started.
    pub(crate) source_id: SourceId,
    /// Focused relative path expected to still be selected on apply.
    pub(crate) relative_path: PathBuf,
    /// Computed highlight payload or the terminal error.
    pub(crate) result: Result<Option<FocusedSimilarityPaths>, String>,
}

/// Async result for one follow-loaded similarity query build request.
#[derive(Debug)]
pub(crate) struct LoadedSimilarityQueryResult {
    /// Monotonic request identifier used to discard stale async results.
    pub(crate) request_id: u64,
    /// Source that owned the loaded sample when the request started.
    pub(crate) source_id: SourceId,
    /// Loaded relative path expected to still be active on apply.
    pub(crate) relative_path: PathBuf,
    /// Browser snapshot key the built query still aligns with.
    pub(crate) key: crate::app::controller::FeatureCacheKey,
    /// Similarity query payload plus retained source snapshot or the terminal error.
    pub(crate) result: Result<crate::app::controller::state::runtime::LoadedSimilarityQueryData, String>,
}

#[derive(Debug)]
pub(crate) struct AnalysisFailuresResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<std::collections::HashMap<PathBuf, String>, String>,
}

#[derive(Debug)]
pub(crate) struct NormalizationJob {
    pub(crate) source: crate::sample_sources::SampleSource,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct NormalizationResult {
    pub(crate) source_id: crate::sample_sources::SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) result: Result<
        (
            u64,
            i64,
            crate::sample_sources::Rating,
            crate::app::controller::undo::OverwriteBackup,
        ),
        String,
    >,
}

/// Startup-deferred source DB maintenance request.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceJob {
    /// Source id used for status/error attribution.
    pub(crate) source_id: SourceId,
    /// Root path of the source database.
    pub(crate) source_root: PathBuf,
}

/// Summary for one source DB maintenance attempt.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceOutcome {
    /// Source id associated with this outcome.
    pub(crate) source_id: SourceId,
    /// Source root used for maintenance.
    pub(crate) source_root: PathBuf,
    /// Whether this source was skipped due to unchanged revision/schema token.
    pub(crate) skipped: bool,
    /// Number of orphaned analysis rows removed.
    pub(crate) orphan_rows_removed: usize,
    /// Whether maintenance changed source-visible DB state and the browser should refresh.
    pub(crate) refresh_required: bool,
    /// Error when maintenance failed after retry attempts.
    pub(crate) error: Option<String>,
}

/// Batched result for deferred source DB maintenance.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceResult {
    /// Per-source maintenance outcomes.
    pub(crate) outcomes: Vec<SourceDbMaintenanceOutcome>,
}
