//! Runtime state and job coordination for the controller.

/// Incremental derived-state dirty graph model used by native projection paths.
mod derived_graph;

use crate::app::controller::jobs;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::sample_sources::db::SourceDbError;
use crate::sample_sources::{ScanMode, SourceId, WavEntry};
pub(crate) use derived_graph::{DerivedNodeId, DerivedStateGraph, DirtyReason};
use rusqlite::Connection;
use std::collections::HashMap;
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
    /// Map selection revision is dirty.
    pub(crate) const MAP_SELECTION: u16 = 1 << 3;
    /// Map hover revision is dirty.
    pub(crate) const MAP_HOVER: u16 = 1 << 4;
    /// Map dataset identity revision is dirty.
    pub(crate) const MAP_DATASET: u16 = 1 << 5;
    /// Map query-bounds revision is dirty.
    pub(crate) const MAP_QUERY: u16 = 1 << 6;
    /// Update panel revision is dirty.
    pub(crate) const UPDATE: u16 = 1 << 7;
    /// Loaded wav path revision is dirty.
    pub(crate) const LOADED_WAV: u16 = 1 << 8;
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
    /// True when a waveform image rebuild is queued for the next frame prep.
    pub(crate) waveform_refresh_pending: bool,
    /// Last known cause for a queued waveform refresh request.
    pub(crate) waveform_refresh_pending_reason: Option<WaveformRefreshReason>,
    /// Nesting depth for waveform refresh batching.
    pub(crate) waveform_refresh_batch_depth: u16,
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
            waveform_refresh_pending: false,
            waveform_refresh_pending_reason: None,
            waveform_refresh_batch_depth: 0,
            derived_graph: DerivedStateGraph::new(),
            pending_age_update_commit: None,
            pending_age_update_commit_not_before: None,
            pending_similarity_refresh: None,
            pending_similarity_refresh_not_before: None,
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

/// Deferred focused-similarity refresh request for the current browser selection.
#[derive(Clone, Debug)]
pub(crate) struct PendingFocusedSimilarityRefresh {
    /// Sample id used to query near-duplicate highlights.
    pub(crate) sample_id: String,
    /// Selected relative wav path expected to still be focused when flushing.
    pub(crate) relative_path: PathBuf,
    /// Optional absolute entry index for the focused row.
    pub(crate) anchor_index: Option<usize>,
}

/// Deferred source-analysis metadata write queued after waveform load completes.
#[derive(Clone, Debug)]
pub(crate) struct PendingLoadedDurationMetadata {
    /// Source id used to construct a stable sample id.
    pub(crate) source_id: SourceId,
    /// Source root used to open the per-source analysis database.
    pub(crate) source_root: PathBuf,
    /// Relative sample path for the loaded waveform.
    pub(crate) relative_path: PathBuf,
    /// Loaded waveform duration in seconds.
    pub(crate) duration_seconds: f32,
    /// Loaded waveform sample rate in Hz.
    pub(crate) sample_rate: u32,
    /// Cached long-sample mark when this path is still selected.
    pub(crate) long_sample_mark: Option<bool>,
}

pub(crate) struct PerformanceGovernorState {
    /// Last user interaction timestamp used for governor hysteresis.
    pub(crate) last_user_activity_at: Option<Instant>,
    /// Most recent slow-frame timestamp used to raise worker priority.
    pub(crate) last_slow_frame_at: Option<Instant>,
    /// Last frame timestamp for inter-frame interval sampling.
    pub(crate) last_frame_at: Option<Instant>,
    /// Smoothed frame time in milliseconds.
    pub(crate) avg_frame_ms: f64,
    /// Number of valid frame samples captured so far.
    pub(crate) frame_sample_count: u32,
    pub(crate) last_worker_count: Option<u32>,
    pub(crate) idle_worker_override: Option<u32>,
}

impl PerformanceGovernorState {
    pub(crate) fn new() -> Self {
        Self {
            last_user_activity_at: None,
            last_slow_frame_at: None,
            last_frame_at: None,
            avg_frame_ms: 0.0,
            frame_sample_count: 0,
            last_worker_count: None,
            idle_worker_override: None,
        }
    }

    /// Update moving-frame metrics from one inter-frame duration sample.
    ///
    /// Uses an EWMA-style filter to keep short-term spikes from dominating the average.
    pub(crate) fn observe_frame_interval(&mut self, frame_interval: Duration) {
        let frame_ms = frame_interval.as_secs_f64() * 1_000.0;
        if frame_ms <= 0.0 {
            return;
        }
        if self.frame_sample_count == 0 {
            self.avg_frame_ms = frame_ms;
            self.frame_sample_count = 1;
            return;
        }
        const FRAME_RATE_ALPHA: f64 = 0.2;
        self.avg_frame_ms = self
            .avg_frame_ms
            .mul_add(1.0 - FRAME_RATE_ALPHA, frame_ms * FRAME_RATE_ALPHA);
        self.frame_sample_count = self.frame_sample_count.saturating_add(1);
    }

    /// Return the averaged frame rate across collected frame-time samples.
    pub(crate) fn average_fps(&self) -> Option<f64> {
        if self.avg_frame_ms <= 0.0 || self.frame_sample_count == 0 {
            return None;
        }
        Some(1_000.0 / self.avg_frame_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::PerformanceGovernorState;
    use std::time::Duration;

    #[test]
    fn average_fps_is_none_before_samples() {
        let state = PerformanceGovernorState::new();
        assert!(state.average_fps().is_none());
        assert_eq!(state.frame_sample_count, 0);
        assert_eq!(state.avg_frame_ms, 0.0);
    }

    #[test]
    fn observe_frame_interval_initializes_average() {
        let mut state = PerformanceGovernorState::new();
        state.observe_frame_interval(Duration::from_millis(16));
        assert_eq!(state.frame_sample_count, 1);
        assert_eq!(state.avg_frame_ms, 16.0);
        assert!((state.average_fps().expect("fps") - 62.5).abs() < f64::EPSILON);
    }

    #[test]
    fn observe_frame_interval_skips_non_positive_samples() {
        let mut state = PerformanceGovernorState::new();
        state.observe_frame_interval(Duration::ZERO);
        state.observe_frame_interval(Duration::from_nanos(500));
        assert_eq!(state.frame_sample_count, 1);
        assert!(state.avg_frame_ms > 0.0);
    }

    #[test]
    fn observe_frame_interval_uses_ewma_update() {
        let mut state = PerformanceGovernorState::new();
        state.observe_frame_interval(Duration::from_millis(10));
        state.observe_frame_interval(Duration::from_millis(20));
        let expected = 12.0;
        assert!((state.avg_frame_ms - expected).abs() < 1e-9);
        assert_eq!(state.frame_sample_count, 2);
    }
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
