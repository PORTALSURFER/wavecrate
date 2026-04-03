use crate::app::controller::FeatureCacheKey;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::audio::PendingPlayback;
use crate::sample_sources::SourceId;
use std::path::PathBuf;
use std::time::Instant;

/// Deferred browser-focus side effects scheduled after immediate selection updates.
#[derive(Clone, Debug)]
pub(crate) struct PendingBrowserFocusCommit {
    /// Source that owned the committed browser focus when it was queued.
    pub(crate) source_id: SourceId,
    /// Relative wav path that must still be selected when the deferred work runs.
    pub(crate) relative_path: PathBuf,
    /// Absolute wav entry index expected to still own browser focus.
    pub(crate) entry_index: usize,
    /// Whether the focused path should be written into browser/random history.
    pub(crate) record_focus_history: bool,
    /// Whether focused-similarity refresh should be scheduled once the commit settles.
    pub(crate) refresh_similarity_highlight: bool,
    /// Whether the focused path still needs a committed audio load dispatch.
    pub(crate) queue_audio_load: bool,
    /// Playback intent captured during the immediate focus commit phase.
    pub(crate) pending_playback: Option<PendingPlayback>,
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

/// In-flight focused-similarity highlight query owned by a background worker.
#[derive(Clone, Debug)]
pub(crate) struct PendingFocusedSimilarityQuery {
    /// Monotonic request identifier used to drop stale async results.
    pub(crate) request_id: u64,
    /// Source that owned the focused sample when the query started.
    pub(crate) source_id: SourceId,
    /// Focused relative wav path expected to still be selected on apply.
    pub(crate) relative_path: PathBuf,
}

/// In-flight follow-loaded similarity query owned by a background worker.
#[derive(Clone, Debug)]
pub(crate) struct PendingLoadedSimilarityQuery {
    /// Monotonic request identifier used to drop stale async results.
    pub(crate) request_id: u64,
    /// Source that owned the loaded sample when the query started.
    pub(crate) source_id: SourceId,
    /// Loaded relative wav path expected to still be active on apply.
    pub(crate) relative_path: PathBuf,
}

/// Pending manual similarity-filter rebuild waiting for wav-entry reload to finish.
#[derive(Clone, Debug)]
pub(crate) struct PendingSimilarityFilterRebuild {
    /// Source that owned the similarity filter when it was scheduled.
    pub(crate) source_id: SourceId,
    /// Relative path that should anchor the rebuilt similarity filter.
    pub(crate) anchor_relative_path: PathBuf,
}

/// Cached selected-source analysis progress data reused across controller frames.
#[derive(Clone, Debug, Default)]
pub(crate) struct AnalysisProgressUiCache {
    /// Source id that owns the cached progress snapshot.
    pub(crate) source_id: Option<SourceId>,
    /// Last source-scoped progress snapshot used for the overlay.
    pub(crate) scoped_progress: Option<analysis_jobs::AnalysisProgress>,
    /// When the scoped progress snapshot was last refreshed from a worker or DB.
    pub(crate) scoped_progress_refreshed_at: Option<Instant>,
    /// Last snapshot of running jobs shown in the overlay.
    pub(crate) running_jobs: Vec<crate::app::state::RunningJobSnapshot>,
    /// When the running-job snapshot list was last refreshed from the DB.
    pub(crate) running_jobs_refreshed_at: Option<Instant>,
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

/// In-flight browser feature-cache refresh owned by the controller.
#[derive(Clone, Debug)]
pub(crate) struct PendingBrowserFeatureCacheRefresh {
    /// Monotonic request identifier used to discard stale results.
    pub(crate) request_id: u64,
    /// Source that owned the wav-entry snapshot when the refresh was queued.
    pub(crate) source_id: SourceId,
    /// Wav-entry snapshot key the refresh rows must still match on apply.
    pub(crate) key: FeatureCacheKey,
}
