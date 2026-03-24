//! Background selection-export job DTOs shared between controller actions and workers.

use super::*;
use crate::sample_sources::Rating;
use crate::sample_sources::WavEntry;
use crate::selection::SelectionRange;
use crate::waveform::DecodedWaveform;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Audio payload captured on the UI thread for one selection-export worker job.
#[derive(Clone, Debug)]
pub(crate) enum SelectionExportAudioPayload {
    /// Reuse full decoded samples already resident in memory.
    Decoded {
        /// Interleaved waveform samples.
        samples: Arc<[f32]>,
        /// Audio channel count.
        channels: u16,
        /// Sample rate in Hz.
        sample_rate: u32,
    },
    /// Fall back to decoding the original file bytes off the UI thread.
    Encoded {
        /// Sanitized WAV bytes for the loaded sample.
        bytes: Arc<[u8]>,
    },
}

/// Immutable snapshot of the loaded selection-export source captured on the UI thread.
#[derive(Clone, Debug)]
pub(crate) struct SelectionExportSnapshot {
    /// Source id that owns the loaded sample.
    pub(crate) source_id: SourceId,
    /// Source root folder.
    pub(crate) source_root: PathBuf,
    /// Relative path of the loaded sample.
    pub(crate) relative_path: PathBuf,
    /// Selection bounds to export.
    pub(crate) bounds: SelectionRange,
    /// Captured audio payload.
    pub(crate) audio: SelectionExportAudioPayload,
    /// Whether short edge fades should be applied to the new file.
    pub(crate) apply_edge_fades: bool,
    /// Edge-fade duration in milliseconds when enabled.
    pub(crate) edge_fade_ms: f32,
    /// Tag to assign to the written clip.
    pub(crate) target_tag: Option<Rating>,
    /// Whether loop metadata should be persisted for the new clip.
    pub(crate) looped: bool,
    /// BPM metadata to persist when the clip is looped.
    pub(crate) bpm: Option<f32>,
}

/// Immutable snapshot of one slice-batch export captured on the UI thread.
#[derive(Clone, Debug)]
pub(crate) struct SelectionSliceBatchExportSnapshot {
    /// Source id that owns the loaded sample.
    pub(crate) source_id: SourceId,
    /// Source root folder.
    pub(crate) source_root: PathBuf,
    /// Relative path of the loaded sample.
    pub(crate) relative_path: PathBuf,
    /// Slice bounds to export as individual clips.
    pub(crate) slices: Vec<SelectionRange>,
    /// Naming profile for the generated clips.
    pub(crate) profile: crate::app::state::WaveformSliceBatchProfile,
    /// Captured audio payload.
    pub(crate) audio: SelectionExportAudioPayload,
    /// Whether short edge fades should be applied to new files.
    pub(crate) apply_edge_fades: bool,
    /// Edge-fade duration in milliseconds when enabled.
    pub(crate) edge_fade_ms: f32,
}

/// Destination-specific job configuration for selection clip exports.
#[derive(Clone, Debug)]
pub(crate) enum SelectionClipDestination {
    /// Register the new clip in the browser, optionally under a folder override.
    Browser {
        /// Keep waveform focus on the original source sample after completion.
        keep_source_focused: bool,
        /// Optional source-relative folder override.
        folder_override: Option<PathBuf>,
    },
    /// Save the clip into one source-relative folder and register it in the browser.
    Folder {
        /// Destination folder relative to the source root.
        folder: PathBuf,
        /// Keep waveform focus on the original source sample after completion.
        keep_source_focused: bool,
    },
    /// Prepare a clip for OS-level external dragging.
    ExternalDrag,
}

/// Playback state preserved before queueing crop-to-new-sample work.
#[derive(Clone, Debug)]
pub(crate) struct SelectionExportPlaybackState {
    /// Whether the source sample was playing when the crop action started.
    pub(crate) was_playing: bool,
    /// Whether playback was looped.
    pub(crate) was_looping: bool,
    /// Optional playback restart position in normalized units.
    pub(crate) start_override: Option<f64>,
}

/// Worker job for non-blocking waveform selection exports.
#[derive(Clone, Debug)]
pub(crate) enum SelectionExportJob {
    /// Export the selection as a new clip.
    Clip {
        /// Monotonic request identifier for stale-result protection.
        request_id: u64,
        /// Captured export snapshot.
        snapshot: SelectionExportSnapshot,
        /// Destination behavior for the new clip.
        destination: SelectionClipDestination,
    },
    /// Crop the selection into a new sample file and sync its DB row.
    CropNewSample {
        /// Monotonic request identifier for stale-result protection.
        request_id: u64,
        /// Captured export snapshot.
        snapshot: SelectionExportSnapshot,
        /// Preserved playback state for post-completion auto-open behavior.
        playback: SelectionExportPlaybackState,
    },
    /// Export the current waveform slice batch as multiple clips.
    SliceBatch {
        /// Monotonic request identifier for stale-result protection.
        request_id: u64,
        /// Captured slice-batch snapshot.
        snapshot: SelectionSliceBatchExportSnapshot,
    },
}

impl SelectionExportJob {
    /// Return the stable request id associated with this job.
    pub(crate) fn request_id(&self) -> u64 {
        match self {
            Self::Clip { request_id, .. }
            | Self::CropNewSample { request_id, .. }
            | Self::SliceBatch { request_id, .. } => *request_id,
        }
    }
}

/// Stage timings captured by one selection-export worker execution.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SelectionExportTimings {
    /// Time spent preparing cropped samples from the captured payload.
    pub(crate) prepare: Duration,
    /// Time spent writing the new file to disk.
    pub(crate) write: Duration,
    /// Time spent synchronizing source DB metadata.
    pub(crate) register: Duration,
    /// Total worker runtime.
    pub(crate) total: Duration,
}

/// Successful completion payload for one selection clip export.
#[derive(Clone, Debug)]
pub(crate) struct SelectionClipExportSuccess {
    /// Request id echoed from the originating job.
    pub(crate) request_id: u64,
    /// Source id that owns the new clip.
    pub(crate) source_id: SourceId,
    /// Source root that owns the new clip.
    pub(crate) source_root: PathBuf,
    /// Newly created browser/source entry.
    pub(crate) entry: WavEntry,
    /// Absolute file path of the created clip.
    pub(crate) absolute_path: PathBuf,
    /// Destination behavior used for the export.
    pub(crate) destination: SelectionClipDestination,
    /// Timings recorded by the worker.
    pub(crate) timings: SelectionExportTimings,
}

/// Successful completion payload for crop-to-new-sample exports.
#[derive(Clone, Debug)]
pub(crate) struct SelectionCropExportSuccess {
    /// Request id echoed from the originating job.
    pub(crate) request_id: u64,
    /// Source id that owns the new clip.
    pub(crate) source_id: SourceId,
    /// Source root that owns the new clip.
    pub(crate) source_root: PathBuf,
    /// Original source-relative path that was cropped.
    pub(crate) source_relative_path: PathBuf,
    /// Newly created browser/source entry.
    pub(crate) entry: WavEntry,
    /// Absolute file path of the created clip.
    pub(crate) absolute_path: PathBuf,
    /// Source tag copied onto the new clip.
    pub(crate) tag: Rating,
    /// Preserved playback state from when the crop was requested.
    pub(crate) playback: SelectionExportPlaybackState,
    /// Timings recorded by the worker.
    pub(crate) timings: SelectionExportTimings,
}

/// Successful completion payload for one slice-batch export.
#[derive(Clone, Debug)]
pub(crate) struct SelectionSliceBatchExportSuccess {
    /// Request id echoed from the originating job.
    pub(crate) request_id: u64,
    /// Source id that owns the new clips.
    pub(crate) source_id: SourceId,
    /// Source root that owns the new clips.
    pub(crate) source_root: PathBuf,
    /// Original source-relative path that was sliced.
    pub(crate) source_relative_path: PathBuf,
    /// Newly created browser/source entries.
    pub(crate) entries: Vec<WavEntry>,
    /// Per-slice errors encountered during the batch, if any.
    pub(crate) errors: Vec<String>,
    /// Timings recorded by the worker.
    pub(crate) timings: SelectionExportTimings,
}

/// Completion result published back to the controller from selection-export workers.
#[derive(Clone, Debug)]
pub(crate) enum SelectionExportResult {
    /// Clip export completion.
    Clip {
        /// Request id echoed from the originating job.
        request_id: u64,
        /// Worker result payload.
        result: Result<SelectionClipExportSuccess, String>,
    },
    /// Crop-to-new-sample completion.
    CropNewSample {
        /// Request id echoed from the originating job.
        request_id: u64,
        /// Worker result payload.
        result: Result<SelectionCropExportSuccess, String>,
    },
    /// Slice-batch export completion.
    SliceBatch {
        /// Request id echoed from the originating job.
        request_id: u64,
        /// Worker result payload.
        result: Result<SelectionSliceBatchExportSuccess, String>,
    },
}

impl SelectionExportResult {
    /// Return the request id associated with this completion, when available.
    pub(crate) fn request_id(&self) -> u64 {
        match self {
            Self::Clip { request_id, .. }
            | Self::CropNewSample { request_id, .. }
            | Self::SliceBatch { request_id, .. } => *request_id,
        }
    }
}

/// Streamed progress or completion message for selection-export work.
#[derive(Clone, Debug)]
pub(crate) enum SelectionExportMessage {
    /// Incremental progress update for a slice-batch export.
    Progress {
        /// Request id echoed from the originating job.
        request_id: u64,
        /// Total number of slices scheduled for export.
        total: usize,
        /// Number of slices completed so far.
        completed: usize,
        /// Optional per-slice detail label.
        detail: Option<String>,
    },
    /// Final completion payload for any selection-export job.
    Finished(SelectionExportResult),
}

/// Return the best audio payload for selection-export work from the currently loaded waveform.
pub(crate) fn build_selection_export_audio_payload(
    decoded: Option<&Arc<DecodedWaveform>>,
    bytes: Arc<[u8]>,
) -> SelectionExportAudioPayload {
    if let Some(decoded) = decoded
        && decoded.peaks.is_none()
        && !decoded.samples.is_empty()
    {
        return SelectionExportAudioPayload::Decoded {
            samples: Arc::clone(&decoded.samples),
            channels: decoded.channels.max(1),
            sample_rate: decoded.sample_rate.max(1),
        };
    }
    SelectionExportAudioPayload::Encoded { bytes }
}
