//! Internal state and constructor DTOs for controller job orchestration.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct PendingFolderScan {
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
}

/// In-flight waveform slice-batch export tracked for UI gating and stale-result checks.
#[derive(Clone, Debug)]
pub(crate) struct PendingSliceBatchExport {
    /// Monotonic request identifier for the active batch.
    pub(crate) request_id: u64,
    /// Source that owns the waveform being exported.
    pub(crate) source_id: SourceId,
    /// Relative path of the waveform being exported.
    pub(crate) relative_path: PathBuf,
}

/// Monotonic request-id counters for async controller jobs.
#[derive(Clone, Copy, Debug)]
pub(super) struct JobRequestCounters {
    pub(crate) next_audio_request_id: u64,
    pub(crate) next_recording_waveform_request_id: u64,
    pub(crate) next_folder_scan_request_id: u64,
    pub(crate) next_similarity_request_id: u64,
    pub(crate) next_selection_export_request_id: u64,
}

impl Default for JobRequestCounters {
    /// Initialize request counters at `1` to avoid sentinel `0` ids.
    fn default() -> Self {
        Self {
            next_audio_request_id: 1,
            next_recording_waveform_request_id: 1,
            next_folder_scan_request_id: 1,
            next_similarity_request_id: 1,
            next_selection_export_request_id: 1,
        }
    }
}

/// In-progress flags for one-shot and stream-based background controller jobs.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct JobInProgressState {
    pub(crate) scan: bool,
    pub(crate) trash_move: bool,
    pub(crate) file_ops: bool,
    pub(crate) umap_build: bool,
    pub(crate) umap_cluster_build: bool,
    pub(crate) update_check: bool,
    pub(crate) issue_gateway: bool,
    pub(crate) issue_gateway_auth: bool,
    pub(crate) issue_gateway_poll: bool,
    pub(crate) issue_token_load: bool,
    pub(crate) issue_token_save: bool,
    pub(crate) issue_token_delete: bool,
    pub(crate) source_db_maintenance: bool,
}

/// Cooperative cancellation handles for long-running controller background jobs.
#[derive(Clone, Debug, Default)]
pub(super) struct JobCancelHandles {
    pub(crate) scan: Option<Arc<AtomicBool>>,
    pub(crate) folder_scan: Option<Arc<AtomicBool>>,
    pub(crate) trash_move: Option<Arc<AtomicBool>>,
    pub(crate) file_ops: Option<Arc<AtomicBool>>,
    pub(crate) issue_gateway_poll: Option<Arc<AtomicBool>>,
}

/// Constructor inputs for [`ControllerJobs`].
pub(crate) struct ControllerJobsInit {
    pub(crate) wav_job_tx: Sender<WavLoadJob>,
    pub(crate) wav_job_rx: Receiver<WavLoadResult>,
    pub(crate) wav_loader: WavLoaderHandle,
    pub(crate) audio_job_tx: Sender<AudioLoadJob>,
    pub(crate) audio_job_rx: Receiver<AudioLoadResult>,
    pub(crate) audio_loader: AudioLoaderHandle,
    pub(crate) recording_waveform_job_tx: RecordingWaveformJobSender,
    pub(crate) recording_waveform_job_rx: Receiver<RecordingWaveformLoadResult>,
    pub(crate) recording_waveform_loader: RecordingWaveformWorkerHandle,
    pub(crate) search_job_tx:
        crate::app::controller::library::wavs::browser_search_worker::SearchJobSender,
    pub(crate) search_job_rx: Receiver<SearchResult>,
    pub(crate) search_worker:
        crate::app::controller::library::wavs::browser_search_worker::SearchWorkerHandle,
    pub(crate) job_message_queue_capacity: usize,
}
