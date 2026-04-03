//! Runtime state and job coordination for the controller.

mod deferred;
/// Incremental derived-state dirty graph model used by native projection paths.
mod derived_graph;
mod performance;

use crate::app::controller::jobs;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::app::state::FolderPaneId;
use crate::sample_sources::Rating;
use crate::sample_sources::db::SourceDbError;
use crate::sample_sources::{ScanMode, SourceId, WavEntry};
pub(crate) use deferred::{
    AnalysisProgressUiCache, PendingFocusedSimilarityQuery, PendingFocusedSimilarityRefresh,
    PendingLoadedDurationMetadata, PendingLoadedSimilarityQuery, PendingSimilarityFilterRebuild,
};
pub(crate) use derived_graph::{DerivedNodeId, DerivedStateGraph, DirtyReason};
pub(crate) use performance::PerformanceGovernorState;
use rusqlite::Connection;
use std::collections::{BTreeSet, HashMap, HashSet};
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
    /// Incremental derived-state dirty graph used by native projection paths.
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
    /// Active async follow-loaded similarity query computation awaiting apply.
    pub(crate) pending_loaded_similarity_query: Option<PendingLoadedSimilarityQuery>,
    /// Pending manual similarity-filter rebuild scheduled after destructive wav mutations.
    pub(crate) pending_similarity_filter_rebuild: Option<PendingSimilarityFilterRebuild>,
    /// Cached selected-source analysis progress metadata for progress-overlay updates.
    pub(crate) analysis_progress_ui: AnalysisProgressUiCache,
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
    /// Active source hydration currently preparing the browser-driving source snapshot.
    pub(crate) pending_active_source_hydration: Option<PendingSourceHydration>,
    /// Inactive-pane source hydration currently preparing one retained folder snapshot.
    pub(crate) pending_inactive_source_hydration: Option<PendingSourceHydration>,
    /// Pending pane-scoped folder projection jobs keyed by owning sidebar pane.
    pub(crate) pending_folder_projections: HashMap<FolderPaneId, PendingFolderProjection>,
    /// Controller-owned metadata writes awaiting background completion by request id.
    pub(crate) pending_metadata_mutations: HashMap<u64, PendingMetadataMutation>,
    /// Relative sample paths currently carrying optimistic metadata writes.
    pub(crate) pending_metadata_paths: HashSet<(SourceId, PathBuf)>,
    /// Source ids currently owning background file or folder mutations.
    pub(crate) pending_file_mutation_sources: HashSet<SourceId>,
    /// Relative paths currently carrying background file or folder mutations.
    pub(crate) pending_file_mutation_paths: HashSet<(SourceId, PathBuf)>,
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
            derived_graph: DerivedStateGraph::new(),
            pending_age_update_commit: None,
            pending_age_update_commit_not_before: None,
            pending_similarity_refresh: None,
            pending_similarity_refresh_not_before: None,
            pending_focused_similarity_query: None,
            pending_loaded_similarity_query: None,
            pending_similarity_filter_rebuild: None,
            analysis_progress_ui: AnalysisProgressUiCache::default(),
            pending_loaded_duration_metadata: None,
            pending_loaded_duration_metadata_not_before: None,
            pending_waveform_seek_nanos: None,
            pending_waveform_seek_not_before: None,
            map_query_connections: HashMap::new(),
            projection_revision_dirty: ProjectionRevisionDirtyMask::default(),
            next_waveform_image_signature: 1,
            delete_recovery_started: false,
            active_retained_delete_resolution: None,
            deferred_startup_source_db_maintenance_jobs: Vec::new(),
            deferred_startup_source_db_maintenance_armed: false,
            startup_frame_prepare_count: 0,
            pending_active_source_hydration: None,
            pending_inactive_source_hydration: None,
            pending_folder_projections: HashMap::new(),
            pending_metadata_mutations: HashMap::new(),
            pending_metadata_paths: HashSet::new(),
            pending_file_mutation_sources: HashSet::new(),
            pending_file_mutation_paths: HashSet::new(),
            #[cfg(test)]
            progress_cancel_after: None,
            #[cfg(test)]
            fail_next_folder_delete_db: false,
            #[cfg(test)]
            fail_after_folder_delete_stage: false,
            #[cfg(test)]
            fail_after_folder_delete_db_commit: false,
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

/// One optimistic metadata request awaiting background persistence.
#[derive(Clone, Debug)]
pub(crate) struct PendingMetadataMutation {
    /// Request id used to match completion messages.
    pub(crate) request_id: u64,
    /// Source that owns the optimistic metadata updates.
    pub(crate) source_id: SourceId,
    /// Paths touched by this request for pending-state cleanup.
    pub(crate) paths: BTreeSet<PathBuf>,
    /// Rollback entries applied only when the background write fails.
    pub(crate) rollback: Vec<MetadataRollback>,
    /// Whether the browser filter/sort projection should refresh when the write completes.
    pub(crate) refresh_browser_projection: bool,
}

/// Rollback payload for one optimistic metadata update.
#[derive(Clone, Debug)]
pub(crate) enum MetadataRollback {
    /// Restore one tag plus keep-lock state if the optimistic value is still current.
    TagAndLocked {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_tag: Rating,
        /// Lock state before the optimistic mutation.
        before_locked: bool,
        /// Value written optimistically before persistence completed.
        expected_tag: Rating,
        /// Lock state written optimistically before persistence completed.
        expected_locked: bool,
    },
    /// Restore one loop-marker state if the optimistic value is still current.
    Looped {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_looped: bool,
        /// Value written optimistically before persistence completed.
        expected_looped: bool,
    },
    /// Restore one playback-age value if the optimistic value is still current.
    LastPlayedAt {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_last_played_at: Option<i64>,
        /// Value written optimistically before persistence completed.
        expected_last_played_at: Option<i64>,
    },
    /// Restore one BPM value if the optimistic value is still current.
    Bpm {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Value before the optimistic mutation.
        before_bpm: Option<f32>,
        /// Value written optimistically before persistence completed.
        expected_bpm: Option<f32>,
    },
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

/// Active controller-side tracking for one source hydration request.
#[derive(Clone, Debug)]
pub(crate) struct PendingSourceHydration {
    /// Monotonic request identifier used to discard stale results.
    pub(crate) request_id: u64,
    /// Sidebar pane that owns the source assignment.
    pub(crate) pane: FolderPaneId,
    /// Hydrated source identifier.
    pub(crate) source_id: SourceId,
    /// Logical hydration lane for result application.
    pub(crate) kind: jobs::SourceHydrationKind,
    /// Search request queued after hydration apply, when active-source projection is pending.
    pub(crate) search_request_id: Option<u64>,
    /// Time when the hydration request was queued on the controller thread.
    pub(crate) queued_at: Instant,
}

/// Active controller-side tracking for one pane-scoped folder projection request.
#[derive(Clone, Debug)]
pub(crate) struct PendingFolderProjection {
    /// Monotonic request identifier used to discard stale results.
    pub(crate) request_id: u64,
    /// Sidebar pane whose folder browser rows are being projected.
    pub(crate) pane: FolderPaneId,
    /// Source identifier that owns the folder browser state.
    pub(crate) source_id: SourceId,
    /// Time when the projection request was queued on the controller thread.
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
