//! Runtime state and job coordination for the controller.

mod deferred;
/// Incremental derived-state dirty graph model used by UI projection paths.
mod derived_graph;
mod performance;
mod source_lane;

use crate::app::controller::jobs;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::sample_sources::db::SourceDbError;
use crate::sample_sources::{ScanMode, SourceId, WavEntry};
pub(crate) use deferred::{
    AnalysisProgressUiCache, BrowserSelectionCommitRequest, BrowserSelectionCommitStage,
    BrowserSelectionLoadState, BrowserSelectionTransition, DeferredStartupAudioRefreshState,
    LoadedSimilarityQueryCache, LoadedSimilarityQueryData, LoadedSimilaritySourceCandidate,
    LoadedSimilaritySourceSnapshot, PendingBrowserFeatureCacheRefresh,
    PendingFocusedSimilarityQuery, PendingFocusedSimilarityRefresh, PendingLoadedDurationMetadata,
    PendingLoadedSimilarityQuery, PendingSimilarityFilterRebuild,
};
pub(crate) use derived_graph::{DerivedNodeId, DerivedStateGraph, DirtyReason};
pub(crate) use performance::PerformanceGovernorState;
use rusqlite::Connection;
#[cfg(test)]
pub(crate) use source_lane::AutoRenameBatchRowSnapshot;
pub(crate) use source_lane::{ActiveAutoRenameBatchSnapshot, AutoRenameBatchRowState};
pub(crate) use source_lane::{
    BrowserRenameBusyDecision, BrowserRenameIntentKey, MetadataRollback,
    PendingBrowserAutoRenameIntent, PendingMetadataMutation, PendingSourceHydration,
    SourceLaneRuntimeState,
};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Classified causes for queued waveform image refresh work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformRefreshReason {
    /// Waveform sample content changed and requires a rerender.
    Data,
    /// Waveform view window/cursor/selection changed.
    View,
    /// Waveform render target dimensions changed.
    Size,
}

/// Bitmask of pending projection revision bumps set at mutation time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProjectionRevisionDirtyMask(pub(crate) u16);

impl ProjectionRevisionDirtyMask {
    /// No pending revision work.
    pub(crate) const NONE: u16 = 0;
    /// Status text/tone revision is dirty.
    pub(crate) const STATUS: u16 = 1 << 0;
    /// Folder-search query revision is dirty.
    pub(crate) const FOLDER_SEARCH: u16 = 1 << 1;
    /// Browser-search query revision is dirty.
    pub(crate) const BROWSER_SEARCH: u16 = 1 << 2;
    /// Browser-row inline metadata revision is dirty.
    pub(crate) const BROWSER_ROW_METADATA: u16 = 1 << 3;
    /// Map selection revision is dirty.
    pub(crate) const MAP_SELECTION: u16 = 1 << 4;
    /// Map hover revision is dirty.
    pub(crate) const MAP_HOVER: u16 = 1 << 5;
    /// Map dataset identity revision is dirty.
    pub(crate) const MAP_DATASET: u16 = 1 << 6;
    /// Map query-bounds revision is dirty.
    pub(crate) const MAP_QUERY: u16 = 1 << 7;
    /// Update panel revision is dirty.
    pub(crate) const UPDATE: u16 = 1 << 8;
    /// Loaded wav path revision is dirty.
    pub(crate) const LOADED_WAV: u16 = 1 << 9;
}

pub(crate) struct ControllerRuntimeState {
    pub(crate) jobs: jobs::ControllerJobs,
    pub(crate) analysis: analysis_jobs::AnalysisWorkerPool,
    pub(crate) performance: PerformanceGovernorState,
    pub(crate) similarity_prep: Option<SimilarityPrepState>,
    pub(crate) similarity_prep_last_error: Option<String>,
    pub(crate) similarity_prep_last_attempt: Option<Instant>,
    pub(crate) similarity_prep_force_full_analysis_next: bool,
    pub(crate) auto_sync_last_by_source: HashMap<SourceId, Instant>,
    /// True when a live volume change is pending persistence.
    pub(crate) volume_persist_dirty: bool,
    /// Debounce deadline for committing a pending volume write.
    pub(crate) volume_persist_deadline: Option<Instant>,
    /// Last persisted volume in milli-units (`0..=1000`).
    pub(crate) last_persisted_volume_milli: Option<u16>,
    /// Active deferred volume/config persistence request, when any.
    pub(crate) pending_config_persist: Option<PendingConfigPersist>,
    /// True when a waveform image rebuild is queued for the next frame prep.
    pub(crate) waveform_refresh_pending: bool,
    /// Last known cause for a queued waveform refresh request.
    pub(crate) waveform_refresh_pending_reason: Option<WaveformRefreshReason>,
    /// Nesting depth for waveform refresh batching.
    pub(crate) waveform_refresh_batch_depth: u16,
    /// Latest queued waveform render request, when any.
    pub(crate) pending_waveform_render: Option<PendingWaveformRender>,
    /// Latest queued waveform transient compute request, when any.
    pub(crate) pending_waveform_transient_compute: Option<PendingWaveformTransientCompute>,
    /// Incremental derived-state dirty graph used by UI projection paths.
    pub(crate) derived_graph: DerivedStateGraph,
    /// Pending playback-age DB update moved out of input action handlers.
    pub(crate) pending_age_update_commit: Option<PendingAgeUpdate>,
    /// Earliest frame time when deferred playback-age persistence may run.
    pub(crate) pending_age_update_commit_not_before: Option<Instant>,
    /// Pending focused-similarity refresh moved out of input action handlers.
    pub(crate) pending_similarity_refresh: Option<PendingFocusedSimilarityRefresh>,
    /// Earliest frame time when deferred focused-similarity refresh may run.
    pub(crate) pending_similarity_refresh_not_before: Option<Instant>,
    /// Active async focused-similarity highlight computation awaiting apply.
    pub(crate) pending_focused_similarity_query: Option<PendingFocusedSimilarityQuery>,
    /// Browser-selection candidate lifecycle spanning preview, commit, loading, and handoff.
    pub(crate) browser_selection_transition: Option<BrowserSelectionTransition>,
    /// Active async follow-loaded similarity query computation awaiting apply.
    pub(crate) pending_loaded_similarity_query: Option<PendingLoadedSimilarityQuery>,
    /// Retained loaded-similarity query cached by source snapshot and anchor sample.
    pub(crate) loaded_similarity_query_cache: Option<LoadedSimilarityQueryCache>,
    /// Pending manual similarity-filter rebuild scheduled after destructive wav mutations.
    pub(crate) pending_similarity_filter_rebuild: Option<PendingSimilarityFilterRebuild>,
    /// Cached selected-source analysis progress metadata for progress-overlay updates.
    pub(crate) analysis_progress_ui: AnalysisProgressUiCache,
    /// Active async browser feature-cache refresh awaiting apply.
    pub(crate) pending_browser_feature_cache_refresh: Option<PendingBrowserFeatureCacheRefresh>,
    /// Pending duration/long-mark metadata write moved out of waveform load hot path.
    pub(crate) pending_loaded_duration_metadata: Option<PendingLoadedDurationMetadata>,
    /// Earliest frame time when deferred duration metadata persistence may run.
    pub(crate) pending_loaded_duration_metadata_not_before: Option<Instant>,
    /// Latest queued waveform seek target from high-frequency interaction updates.
    pub(crate) pending_waveform_seek_nanos: Option<u32>,
    /// Earliest frame time when a deferred waveform seek commit may run.
    pub(crate) pending_waveform_seek_not_before: Option<Instant>,
    /// Reused map-query SQLite connections keyed by source id.
    pub(crate) map_query_connections: HashMap<SourceId, Connection>,
    /// Pending projection revision bumps recorded by mutation paths.
    pub(crate) projection_revision_dirty: ProjectionRevisionDirtyMask,
    /// Source-relative metadata paths that must ride the next async browser-search job.
    pub(crate) pending_browser_search_metadata_delta_paths: BTreeSet<PathBuf>,
    /// Monotonic producer-side id for newly rendered waveform image payloads.
    pub(crate) next_waveform_image_signature: u64,
    /// Tracks whether staged delete recovery has been scheduled for this session.
    pub(crate) delete_recovery_started: bool,
    /// Explicit retained-delete resolution currently running through the file-op lane.
    pub(crate) active_retained_delete_resolution: Option<jobs::ActiveRetainedDeleteResolution>,
    /// Startup-deferred source DB maintenance jobs waiting for background launch.
    pub(crate) deferred_startup_source_db_maintenance_jobs: Vec<jobs::SourceDbMaintenanceJob>,
    /// True when deferred startup source DB maintenance should start after first paint.
    pub(crate) deferred_startup_source_db_maintenance_armed: bool,
    /// Number of prepared frame passes since startup configuration was applied.
    pub(crate) startup_frame_prepare_count: u32,
    /// Startup audio refresh deferred until after the first presented frame.
    pub(crate) deferred_startup_audio_refresh: DeferredStartupAudioRefreshState,
    /// Source-specific runtime state for hydration, folder projection, and mutations.
    pub(crate) source_lane: SourceLaneRuntimeState,
    #[cfg(test)]
    pub(crate) progress_cancel_after: Option<usize>,
    #[cfg(test)]
    /// Force the next folder delete DB write to fail for rollback tests.
    pub(crate) fail_next_folder_delete_db: bool,
    #[cfg(test)]
    /// Simulate a crash after staging a folder delete.
    pub(crate) fail_after_folder_delete_stage: bool,
    #[cfg(test)]
    /// Simulate a crash after committing the folder delete DB update.
    pub(crate) fail_after_folder_delete_db_commit: bool,
    #[cfg(test)]
    /// Force the next waveform-to-browser copy registration to fail after the file copy.
    pub(crate) fail_next_waveform_copy_registration: bool,
}

impl ControllerRuntimeState {
    pub(crate) fn new(
        jobs: jobs::ControllerJobs,
        analysis: analysis_jobs::AnalysisWorkerPool,
    ) -> Self {
        Self {
            jobs,
            analysis,
            performance: PerformanceGovernorState::new(),
            similarity_prep: None,
            similarity_prep_last_error: None,
            similarity_prep_last_attempt: None,
            similarity_prep_force_full_analysis_next: false,
            auto_sync_last_by_source: HashMap::new(),
            volume_persist_dirty: false,
            volume_persist_deadline: None,
            last_persisted_volume_milli: None,
            pending_config_persist: None,
            waveform_refresh_pending: false,
            waveform_refresh_pending_reason: None,
            waveform_refresh_batch_depth: 0,
            pending_waveform_render: None,
            pending_waveform_transient_compute: None,
            derived_graph: DerivedStateGraph::new(),
            pending_age_update_commit: None,
            pending_age_update_commit_not_before: None,
            pending_similarity_refresh: None,
            pending_similarity_refresh_not_before: None,
            pending_focused_similarity_query: None,
            browser_selection_transition: None,
            pending_loaded_similarity_query: None,
            loaded_similarity_query_cache: None,
            pending_similarity_filter_rebuild: None,
            analysis_progress_ui: AnalysisProgressUiCache::default(),
            pending_browser_feature_cache_refresh: None,
            pending_loaded_duration_metadata: None,
            pending_loaded_duration_metadata_not_before: None,
            pending_waveform_seek_nanos: None,
            pending_waveform_seek_not_before: None,
            map_query_connections: HashMap::new(),
            projection_revision_dirty: ProjectionRevisionDirtyMask::default(),
            pending_browser_search_metadata_delta_paths: BTreeSet::new(),
            next_waveform_image_signature: 1,
            delete_recovery_started: false,
            active_retained_delete_resolution: None,
            deferred_startup_source_db_maintenance_jobs: Vec::new(),
            deferred_startup_source_db_maintenance_armed: false,
            startup_frame_prepare_count: 0,
            deferred_startup_audio_refresh: DeferredStartupAudioRefreshState::default(),
            source_lane: SourceLaneRuntimeState::default(),
            #[cfg(test)]
            progress_cancel_after: None,
            #[cfg(test)]
            fail_next_folder_delete_db: false,
            #[cfg(test)]
            fail_after_folder_delete_stage: false,
            #[cfg(test)]
            fail_after_folder_delete_db_commit: false,
            #[cfg(test)]
            fail_next_waveform_copy_registration: false,
        }
    }

    /// Begin a waveform-refresh batch where refresh requests are coalesced.
    pub(crate) fn begin_waveform_refresh_batch(&mut self) {
        self.waveform_refresh_batch_depth = self.waveform_refresh_batch_depth.saturating_add(1);
    }

    /// End the current waveform-refresh batch, saturating at zero depth.
    pub(crate) fn end_waveform_refresh_batch(&mut self) {
        self.waveform_refresh_batch_depth = self.waveform_refresh_batch_depth.saturating_sub(1);
    }

    /// Return true when waveform refresh requests should be deferred.
    pub(crate) fn waveform_refresh_batch_active(&self) -> bool {
        self.waveform_refresh_batch_depth > 0
    }
}

/// Active deferred configuration persistence request.
#[derive(Clone, Debug)]
pub(crate) struct PendingConfigPersist {
    /// Request id used to discard stale completions.
    pub(crate) request_id: u64,
    /// Last queued normalized volume value.
    pub(crate) volume: f32,
    /// Time when the request was queued.
    pub(crate) queued_at: Instant,
}

/// Latest-only waveform render request owned by the controller.
#[derive(Clone, Debug)]
pub(crate) struct PendingWaveformRender {
    /// Request id used to discard stale completions.
    pub(crate) request_id: u64,
    /// Stable render key used for staleness checks.
    pub(crate) key: jobs::WaveformRenderKey,
    /// Time when the render request was queued.
    pub(crate) queued_at: Instant,
}

/// Latest-only waveform transient compute request owned by the controller.
#[derive(Clone, Debug)]
pub(crate) struct PendingWaveformTransientCompute {
    /// Request id used to discard stale completions.
    pub(crate) request_id: u64,
    /// Decode cache token used for staleness checks.
    pub(crate) cache_token: u64,
    /// Time when the transient request was queued.
    pub(crate) queued_at: Instant,
}

#[derive(Clone, Debug)]
pub(crate) struct SimilarityPrepState {
    pub(crate) source_id: SourceId,
    pub(crate) stage: SimilarityPrepStage,
    pub(crate) umap_version: String,
    pub(crate) scan_completed_at: Option<i64>,
    pub(crate) skip_backfill: bool,
    pub(crate) force_full_analysis: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SimilarityPrepStage {
    AwaitScan,
    AwaitEmbeddings,
    Finalizing,
}

pub(crate) struct WavLoadJob {
    pub(crate) source_id: SourceId,
    pub(crate) root: PathBuf,
    pub(crate) page_size: usize,
}

#[derive(Debug)]
pub(crate) struct WavLoadResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<Vec<WavEntry>, LoadEntriesError>,
    pub(crate) elapsed: Duration,
    pub(crate) total: usize,
    pub(crate) page_index: usize,
}

#[derive(Debug)]
pub(crate) struct ScanResult {
    pub(crate) source_id: SourceId,
    pub(crate) mode: ScanMode,
    pub(crate) kind: ScanKind,
    pub(crate) result: Result<
        crate::sample_sources::scanner::ScanStats,
        crate::sample_sources::scanner::ScanError,
    >,
}

/// Indicates whether a scan was triggered by the user or automatically in the background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScanKind {
    Manual,
    Auto,
}

#[derive(Debug)]
pub(crate) enum ScanJobMessage {
    Progress {
        completed: usize,
        detail: Option<String>,
    },
    Finished(ScanResult),
}

#[derive(Clone, Debug)]
pub(crate) struct UpdateCheckResult {
    pub(crate) result: Result<crate::updater::UpdateCheckOutcome, String>,
}

#[derive(Debug)]
pub(crate) enum LoadEntriesError {
    Db(SourceDbError),
    Message(String),
}

impl std::fmt::Display for LoadEntriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadEntriesError::Db(err) => write!(f, "{err}"),
            LoadEntriesError::Message(msg) => f.write_str(msg),
        }
    }
}

impl From<String> for LoadEntriesError {
    fn from(value: String) -> Self {
        LoadEntriesError::Message(value)
    }
}
