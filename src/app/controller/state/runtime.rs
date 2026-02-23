//! Runtime state and job coordination for the controller.

/// Incremental derived-state dirty graph model used by native projection paths.
mod derived_graph;

use crate::app::controller::jobs;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::app::state::{MapQueryBounds, StatusTone, UpdateStatus};
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

/// Bitwise-stable key for map query bounds in projection revision snapshots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MapQueryBoundsRevisionKey {
    /// Bitwise minimum x bound.
    pub(crate) min_x_bits: u32,
    /// Bitwise maximum x bound.
    pub(crate) max_x_bits: u32,
    /// Bitwise minimum y bound.
    pub(crate) min_y_bits: u32,
    /// Bitwise maximum y bound.
    pub(crate) max_y_bits: u32,
}

impl MapQueryBoundsRevisionKey {
    /// Build one snapshot key from floating query bounds.
    pub(crate) fn from_bounds(bounds: MapQueryBounds) -> Self {
        Self {
            min_x_bits: bounds.min_x.to_bits(),
            max_x_bits: bounds.max_x.to_bits(),
            min_y_bits: bounds.min_y.to_bits(),
            max_y_bits: bounds.max_y.to_bits(),
        }
    }
}

/// Last observed controller fields used to bump projection revisions.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ProjectionRevisionSnapshot {
    /// Last observed status message text.
    pub(crate) status_text: String,
    /// Last observed status tone.
    pub(crate) status_tone: Option<StatusTone>,
    /// Last observed folder-search query.
    pub(crate) folder_search_query: String,
    /// Last observed browser-search query.
    pub(crate) browser_search_query: String,
    /// Last observed map selected sample id.
    pub(crate) map_selected_sample_id: Option<String>,
    /// Last observed map hovered sample id.
    pub(crate) map_hovered_sample_id: Option<String>,
    /// Last observed map UMAP version.
    pub(crate) map_umap_version: String,
    /// Last observed map cached bounds source id.
    pub(crate) map_cached_bounds_source_id: Option<String>,
    /// Last observed map cached bounds UMAP version.
    pub(crate) map_cached_bounds_umap_version: Option<String>,
    /// Last observed map cached points source id.
    pub(crate) map_cached_points_source_id: Option<String>,
    /// Last observed map cached points UMAP version.
    pub(crate) map_cached_points_umap_version: Option<String>,
    /// Last observed map query bounds key.
    pub(crate) map_last_query: Option<MapQueryBoundsRevisionKey>,
    /// Last observed update status.
    pub(crate) update_status: Option<UpdateStatus>,
    /// Last observed available update tag.
    pub(crate) update_available_tag: Option<String>,
    /// Last observed available update URL.
    pub(crate) update_available_url: Option<String>,
    /// Last observed update error string.
    pub(crate) update_last_error: Option<String>,
    /// Last observed loaded wav path.
    pub(crate) loaded_wav: Option<PathBuf>,
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
    /// Latest queued waveform seek target from high-frequency interaction updates.
    pub(crate) pending_waveform_seek_milli: Option<u16>,
    /// Earliest frame time when a deferred waveform seek commit may run.
    pub(crate) pending_waveform_seek_not_before: Option<Instant>,
    /// Reused map-query SQLite connections keyed by source id.
    pub(crate) map_query_connections: HashMap<SourceId, Connection>,
    /// Last observed projection-sensitive values for revision bumping.
    pub(crate) projection_revision_snapshot: ProjectionRevisionSnapshot,
    /// Tracks whether staged delete recovery has been scheduled for this session.
    pub(crate) delete_recovery_started: bool,
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
            pending_waveform_seek_milli: None,
            pending_waveform_seek_not_before: None,
            map_query_connections: HashMap::new(),
            projection_revision_snapshot: ProjectionRevisionSnapshot::default(),
            delete_recovery_started: false,
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
