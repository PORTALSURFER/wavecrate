//! Metadata mutation and config-persist DTOs for controller job lanes.

use super::*;

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
    /// Persist one sound-type metadata change.
    SetSoundType {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// New sound type to store, or `None` to clear it.
        sound_type: Option<crate::sample_sources::SampleSoundType>,
    },
    /// Persist one custom user-tag metadata change.
    SetUserTag {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// New user tag to store, or `None` to clear it.
        user_tag: Option<String>,
    },
    /// Persist whether a sample filename is currently tag-derived.
    SetTagNamed {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// New tag-derived filename marker to store.
        tag_named: bool,
    },
    /// Assign one normal library tag to one sample.
    AssignNormalTag {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Tag label to resolve-or-create and assign.
        label: String,
    },
    /// Remove one normal library tag assignment from one sample.
    RemoveNormalTag {
        /// Relative sample path within the source root.
        relative_path: PathBuf,
        /// Tag label to resolve and remove.
        label: String,
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
