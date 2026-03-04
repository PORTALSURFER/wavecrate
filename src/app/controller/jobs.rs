#![allow(clippy::result_large_err)]

/// Issue-gateway and issue-token worker runners.
mod issue_gateway_jobs;
/// Background audio normalization worker helpers.
mod normalization_worker;
/// Worker/result forwarding helpers for progress and completion messages.
mod progress_reporting;
/// Bounded job-queue sender/drop policy helpers.
mod queue_orchestration;
/// Retry/backoff policy helpers for async gateway and maintenance jobs.
mod retry_policy;
/// Deferred source-db maintenance worker helpers.
mod source_db_maintenance;

use self::normalization_worker::run_normalization_job;
use self::progress_reporting::{
    JobForwarderHandles, JobForwarderSpawnConfig, ProgressForwarderConfig, spawn_progress_forwarder,
};
use self::queue_orchestration::new_job_message_queue;
#[cfg(test)]
use self::retry_policy::IssueGatewayPollConfig;
use self::retry_policy::{issue_gateway_poll_config, poll_issue_gateway_with_backoff};
use self::source_db_maintenance::run_source_db_maintenance_job;
use super::ScanJobMessage;
use super::library::analysis_jobs::AnalysisJobMessage;
use super::library::source_folders::delete_recovery::DeleteRecoveryReport;
use super::library::trash_move;
use super::library::wav_entries_loader::WavLoaderHandle;
use super::playback::audio_loader::{AudioLoadJob, AudioLoadResult, AudioLoaderHandle};
use super::playback::recording::waveform_loader::{
    RecordingWaveformJob, RecordingWaveformJobSender, RecordingWaveformLoadResult,
    RecordingWaveformWorkerHandle,
};
use super::source_watcher::{
    SourceWatchCommand, SourceWatchEntry, SourceWatchEvent, SourceWatcherHandle,
};
use super::state::audio::{PendingAudio, PendingPlayback, PendingRecordingWaveform};
use super::state::runtime::{UpdateCheckResult, WavLoadJob, WavLoadResult};
use crate::gui::repaint::{RepaintSignal, SharedRepaintSignal};
use crate::sample_sources::SourceId;
#[cfg(test)]
use std::time::Duration;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread,
};

pub(crate) use self::queue_orchestration::JobMessageSender;

type TryRecvError = std::sync::mpsc::TryRecvError;

#[derive(Debug)]
#[cfg_attr(test, allow(dead_code))]
pub(crate) enum JobMessage {
    WavLoaded(WavLoadResult),
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
    Normalized(NormalizationResult),
}

#[derive(Debug)]
pub(crate) struct SearchJob {
    /// Monotonic request identifier used to discard stale async search results.
    pub(super) request_id: u64,
    pub(super) source_id: SourceId,
    pub(super) source_root: PathBuf,
    pub(super) query: String,
    pub(super) filter: crate::app::state::TriageFlagFilter,
    /// Rating levels selected for filtering (-3..=3). Empty means no rating filter.
    pub(super) rating_filter: BTreeSet<i8>,
    pub(super) sort: crate::app::state::SampleBrowserSort,
    pub(super) similar_query: Option<crate::app::state::SimilarQuery>,
    pub(super) folder_selection: Option<BTreeSet<PathBuf>>,
    pub(super) folder_negated: Option<BTreeSet<PathBuf>>,
    pub(super) root_mode: crate::app::state::RootFolderFilterMode,
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
    pub(super) model_id: String,
    pub(super) umap_version: String,
    pub(super) source_id: SourceId,
}

#[derive(Debug)]
pub(crate) struct UmapBuildResult {
    pub(super) umap_version: String,
    pub(super) result: Result<(), String>,
}

#[derive(Debug, Clone)]
pub(crate) struct UmapClusterBuildJob {
    pub(super) model_id: String,
    pub(super) umap_version: String,
    pub(super) source_id: Option<SourceId>,
}

#[derive(Debug)]
pub(crate) struct UmapClusterBuildResult {
    #[allow(dead_code)]
    pub(super) umap_version: String,
    pub(super) source_id: Option<SourceId>,
    pub(super) result: Result<crate::analysis::hdbscan::HdbscanStats, String>,
}

#[derive(Debug)]
pub(crate) struct SimilarityPrepOutcome {
    pub(crate) cluster_stats: crate::analysis::hdbscan::HdbscanStats,
    #[allow(dead_code)]
    pub(super) umap_version: String,
}

#[derive(Debug)]
pub(crate) struct SimilarityPrepResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<SimilarityPrepOutcome, String>,
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
    pub(crate) result: Result<(u64, i64, crate::sample_sources::Rating), String>,
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
    /// Error when maintenance failed after retry attempts.
    pub(crate) error: Option<String>,
}

/// Batched result for deferred source DB maintenance.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceResult {
    /// Per-source maintenance outcomes.
    pub(crate) outcomes: Vec<SourceDbMaintenanceOutcome>,
}

/// Progress updates for file operations that should not block the UI thread.
#[derive(Debug)]
pub(crate) enum FileOpMessage {
    /// Incremental progress update for the active file operation.
    Progress {
        /// Completed steps so far.
        completed: usize,
        /// Optional per-item detail label.
        detail: Option<String>,
    },
    /// Final result for the file operation.
    Finished(FileOpResult),
}

/// Outcome for a file operation job.
#[derive(Debug)]
pub(crate) enum FileOpResult {
    /// Clipboard paste or import results.
    ClipboardPaste(ClipboardPasteResult),
    /// Source move results from drag/drop actions.
    SourceMove(SourceMoveResult),
    /// In-source sample move results from folder drag/drop actions.
    FolderSampleMove(FolderSampleMoveResult),
    /// Folder move results from drag/drop actions.
    FolderMove(FolderMoveResult),
    /// Undo/redo filesystem results.
    UndoFile(UndoFileOpResult),
}

/// Successful paste into a source folder with metadata for follow-up updates.
#[derive(Debug)]
pub(crate) struct SourcePasteAdded {
    /// Relative path of the added sample within the source root.
    pub(crate) relative_path: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
}

/// Result of pasting or importing files from the clipboard into a target.
#[derive(Debug)]
pub(crate) struct ClipboardPasteResult {
    /// Destination that received the pasted files.
    pub(crate) outcome: ClipboardPasteOutcome,
    /// Number of skipped files that were unsupported or missing.
    pub(crate) skipped: usize,
    /// Errors encountered while processing files.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
    /// Human-readable label for the target destination.
    pub(crate) target_label: String,
    /// Past-tense label for status reporting (e.g., "Pasted", "Imported").
    pub(crate) action_past_tense: &'static str,
}

/// Target-specific clipboard paste outcomes.
#[derive(Debug)]
pub(crate) enum ClipboardPasteOutcome {
    /// Paste into a source folder.
    Source {
        /// Source receiving the files.
        source_id: crate::sample_sources::SourceId,
        /// Added samples with metadata.
        added: Vec<SourcePasteAdded>,
    },
}

/// Request payload for a background source move operation.
#[derive(Debug, Clone)]
pub(crate) struct SourceMoveRequest {
    /// Source identifier for the sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Root folder for the source.
    pub(crate) source_root: PathBuf,
    /// Relative path of the sample to move.
    pub(crate) relative_path: PathBuf,
}

/// Result of a background source move operation.
#[derive(Debug)]
pub(crate) struct SourceMoveResult {
    /// Target source identifier for the move.
    pub(crate) target_source_id: crate::sample_sources::SourceId,
    /// Successful moves with metadata.
    pub(crate) moved: Vec<SourceMoveSuccess>,
    /// Errors encountered during the move.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Record for a successfully moved sample.
#[derive(Debug)]
pub(crate) struct SourceMoveSuccess {
    /// Original source identifier.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Original relative path.
    pub(crate) relative_path: PathBuf,
    /// New relative path at the destination.
    pub(crate) target_relative: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
}

/// Request payload for a background in-source folder sample move.
#[derive(Debug, Clone)]
pub(crate) struct FolderSampleMoveRequest {
    /// Relative path of the sample to move.
    pub(crate) relative_path: PathBuf,
    /// Relative destination path within the same source.
    pub(crate) target_relative: PathBuf,
}

/// Metadata describing a moved entry within a source.
#[derive(Debug, Clone)]
pub(crate) struct FolderEntryMove {
    /// Original relative path before the move.
    pub(crate) old_relative: PathBuf,
    /// New relative path after the move.
    pub(crate) new_relative: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
}

/// Result of a background in-source folder sample move operation.
#[derive(Debug)]
pub(crate) struct FolderSampleMoveResult {
    /// Source identifier for the moved samples.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Successful moves with metadata.
    pub(crate) moved: Vec<FolderEntryMove>,
    /// Errors encountered during the move.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Request payload for a background folder move within a source.
#[derive(Debug, Clone)]
pub(crate) struct FolderMoveRequest {
    /// Source identifier for the folder.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Root folder for the source.
    pub(crate) source_root: PathBuf,
    /// Folder path relative to the source root.
    pub(crate) folder: PathBuf,
    /// Target parent folder relative to the source root.
    pub(crate) target_folder: PathBuf,
}

/// Result of a background folder move within a source.
#[derive(Debug)]
pub(crate) struct FolderMoveResult {
    /// Source identifier for the moved folder.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Original folder path relative to the source root.
    pub(crate) old_folder: PathBuf,
    /// New folder path relative to the source root.
    pub(crate) new_folder: PathBuf,
    /// True when the folder move completed successfully.
    pub(crate) folder_moved: bool,
    /// Successful entry moves with metadata.
    pub(crate) moved: Vec<FolderEntryMove>,
    /// Errors encountered during the move.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Request for a background undo/redo filesystem operation.
#[derive(Debug, Clone)]
pub(crate) enum UndoFileJob {
    /// Overwrite an existing file with a backup copy.
    Overwrite {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Root folder for the source.
        source_root: PathBuf,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// Absolute destination path to overwrite.
        absolute_path: PathBuf,
        /// Backup file to copy from.
        backup_path: PathBuf,
    },
    /// Remove a sample file and drop its database entry.
    RemoveSample {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Root folder for the source.
        source_root: PathBuf,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// Absolute path to delete.
        absolute_path: PathBuf,
    },
    /// Restore a sample file from backup and update its database entry.
    RestoreSample {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Root folder for the source.
        source_root: PathBuf,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// Absolute destination path to restore.
        absolute_path: PathBuf,
        /// Backup file to copy from.
        backup_path: PathBuf,
        /// Tag to apply after restoration.
        tag: crate::sample_sources::Rating,
    },
}

/// Result of a background undo/redo filesystem operation.
#[derive(Debug)]
pub(crate) struct UndoFileOpResult {
    /// Result of the filesystem operation.
    pub(crate) result: Result<UndoFileOutcome, String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Outcome details for an undo/redo filesystem operation.
#[derive(Debug)]
pub(crate) enum UndoFileOutcome {
    /// File overwrite completed with updated metadata.
    Overwrite {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// File size in bytes.
        file_size: u64,
        /// Modified time as epoch nanoseconds.
        modified_ns: i64,
        /// Tag associated with the sample.
        tag: crate::sample_sources::Rating,
        /// Loop marker state.
        looped: bool,
        /// Last played timestamp, if any.
        last_played_at: Option<i64>,
    },
    /// File removal completed.
    Removed {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Relative path of the sample.
        relative_path: PathBuf,
    },
    /// File restoration completed with updated metadata.
    Restored {
        /// Source identifier for the sample.
        source_id: crate::sample_sources::SourceId,
        /// Relative path of the sample.
        relative_path: PathBuf,
        /// File size in bytes.
        file_size: u64,
        /// Modified time as epoch nanoseconds.
        modified_ns: i64,
        /// Tag associated with the sample.
        tag: crate::sample_sources::Rating,
        /// Loop marker state.
        looped: bool,
        /// Last played timestamp, if any.
        last_played_at: Option<i64>,
    },
}

/// Coordinator for controller job channels, worker handles, and job state.
pub(crate) struct ControllerJobs {
    pub(crate) wav_job_tx: Sender<WavLoadJob>,
    pub(crate) audio_job_tx: Sender<AudioLoadJob>,
    recording_waveform_job_tx: RecordingWaveformJobSender,
    pub(crate) search_job_tx:
        crate::app::controller::library::wavs::browser_search_worker::SearchJobSender,
    wav_loader: WavLoaderHandle,
    audio_loader: AudioLoaderHandle,
    recording_waveform_loader: RecordingWaveformWorkerHandle,
    search_worker: crate::app::controller::library::wavs::browser_search_worker::SearchWorkerHandle,
    source_watcher: SourceWatcherHandle,
    forwarders: Option<JobForwarderHandles>,
    message_tx: JobMessageSender,
    message_rx: Receiver<JobMessage>,
    pub(super) pending_source: Option<SourceId>,
    pub(super) pending_select_path: Option<PathBuf>,
    pub(super) pending_audio: Option<PendingAudio>,
    pub(super) pending_playback: Option<PendingPlayback>,
    pub(super) pending_recording_waveform: Option<PendingRecordingWaveform>,
    pub(super) request_counters: JobRequestCounters,
    pub(super) in_progress: JobInProgressState,
    pub(super) cancel_handles: JobCancelHandles,
    pub(super) pending_folder_scan: Option<PendingFolderScan>,
    pub(super) repaint_signal: Arc<SharedRepaintSignal>,
}

#[derive(Clone, Debug)]
pub(super) struct PendingFolderScan {
    request_id: u64,
    source_id: SourceId,
}

/// Monotonic request-id counters for async controller jobs.
#[derive(Clone, Copy, Debug)]
pub(super) struct JobRequestCounters {
    next_audio_request_id: u64,
    next_recording_waveform_request_id: u64,
    next_folder_scan_request_id: u64,
}

impl Default for JobRequestCounters {
    /// Initialize request counters at `1` to avoid sentinel `0` ids.
    fn default() -> Self {
        Self {
            next_audio_request_id: 1,
            next_recording_waveform_request_id: 1,
            next_folder_scan_request_id: 1,
        }
    }
}

/// In-progress flags for one-shot and stream-based background controller jobs.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct JobInProgressState {
    scan: bool,
    trash_move: bool,
    file_ops: bool,
    umap_build: bool,
    umap_cluster_build: bool,
    update_check: bool,
    issue_gateway: bool,
    issue_gateway_auth: bool,
    issue_gateway_poll: bool,
    issue_token_load: bool,
    issue_token_save: bool,
    issue_token_delete: bool,
    source_db_maintenance: bool,
}

/// Cooperative cancellation handles for long-running controller background jobs.
#[derive(Clone, Debug, Default)]
pub(super) struct JobCancelHandles {
    scan: Option<Arc<AtomicBool>>,
    folder_scan: Option<Arc<AtomicBool>>,
    trash_move: Option<Arc<AtomicBool>>,
    file_ops: Option<Arc<AtomicBool>>,
    issue_gateway_poll: Option<Arc<AtomicBool>>,
}

/// Constructor inputs for [`ControllerJobs`].
pub(super) struct ControllerJobsInit {
    pub(super) wav_job_tx: Sender<WavLoadJob>,
    pub(super) wav_job_rx: Receiver<WavLoadResult>,
    pub(super) wav_loader: WavLoaderHandle,
    pub(super) audio_job_tx: Sender<AudioLoadJob>,
    pub(super) audio_job_rx: Receiver<AudioLoadResult>,
    pub(super) audio_loader: AudioLoaderHandle,
    pub(super) recording_waveform_job_tx: RecordingWaveformJobSender,
    pub(super) recording_waveform_job_rx: Receiver<RecordingWaveformLoadResult>,
    pub(super) recording_waveform_loader: RecordingWaveformWorkerHandle,
    pub(super) search_job_tx:
        crate::app::controller::library::wavs::browser_search_worker::SearchJobSender,
    pub(super) search_job_rx: Receiver<SearchResult>,
    pub(super) search_worker:
        crate::app::controller::library::wavs::browser_search_worker::SearchWorkerHandle,
    pub(super) job_message_queue_capacity: usize,
}

impl ControllerJobs {
    /// Build controller job orchestration from pre-spawned worker channels/handles.
    pub(super) fn new(init: ControllerJobsInit) -> Self {
        let ControllerJobsInit {
            wav_job_tx,
            wav_job_rx,
            wav_loader,
            audio_job_tx,
            audio_job_rx,
            audio_loader,
            recording_waveform_job_tx,
            recording_waveform_job_rx,
            recording_waveform_loader,
            search_job_tx,
            search_job_rx,
            search_worker,
            job_message_queue_capacity,
        } = init;
        let (message_tx, message_rx) = new_job_message_queue(job_message_queue_capacity);
        let source_watcher = super::source_watcher::spawn_source_watcher(message_tx.clone());
        let repaint_signal = Arc::new(SharedRepaintSignal::default());
        let forwarders = JobForwarderHandles::spawn(JobForwarderSpawnConfig {
            message_tx: message_tx.clone(),
            repaint_signal: repaint_signal.clone(),
            wav_job_rx,
            audio_job_rx,
            recording_waveform_job_rx,
            search_job_rx,
        });

        Self {
            wav_job_tx,
            audio_job_tx,
            recording_waveform_job_tx,
            search_job_tx,
            wav_loader,
            audio_loader,
            recording_waveform_loader,
            search_worker,
            source_watcher,
            forwarders: Some(forwarders),
            message_tx,
            message_rx,
            pending_source: None,
            pending_select_path: None,
            pending_audio: None,
            pending_playback: None,
            pending_recording_waveform: None,
            request_counters: JobRequestCounters::default(),
            in_progress: JobInProgressState::default(),
            cancel_handles: JobCancelHandles::default(),
            pending_folder_scan: None,
            repaint_signal,
        }
    }

    pub(super) fn try_recv_message(&self) -> Result<JobMessage, TryRecvError> {
        self.message_rx.try_recv()
    }

    pub(super) fn message_sender(&self) -> JobMessageSender {
        self.message_tx.clone()
    }

    pub(crate) fn set_repaint_signal(&self, signal: Arc<dyn RepaintSignal>) {
        self.repaint_signal.set_signal(Some(signal));
    }

    /// Shut down background workers owned by the controller to avoid leaking threads on exit.
    pub(crate) fn shutdown(&mut self) {
        if let Some(cancel) = self.cancel_handles.scan.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(cancel) = self.cancel_handles.folder_scan.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(cancel) = self.cancel_handles.trash_move.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(cancel) = self.cancel_handles.file_ops.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(cancel) = self.cancel_handles.issue_gateway_poll.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.source_watcher.shutdown();
        self.search_worker.shutdown();
        self.recording_waveform_loader.shutdown();
        self.audio_loader.shutdown();
        self.wav_loader.shutdown();
        if let Some(forwarders) = self.forwarders.take() {
            forwarders.join();
        }
    }

    /// Update the source roots watched for on-disk changes.
    pub(crate) fn update_source_watcher(&self, sources: Vec<SourceWatchEntry>) {
        self.source_watcher
            .send(SourceWatchCommand::ReplaceSources(sources));
    }

    pub(super) fn wav_load_pending_for(&self, source_id: &SourceId) -> bool {
        self.pending_source.as_ref() == Some(source_id)
    }

    pub(super) fn mark_wav_load_pending(&mut self, source_id: SourceId) {
        self.pending_source = Some(source_id);
    }

    pub(super) fn clear_wav_load_pending(&mut self) {
        self.pending_source = None;
    }

    pub(super) fn send_wav_job(&self, job: WavLoadJob) {
        let _ = self.wav_job_tx.send(job);
    }

    pub(super) fn set_pending_select_path(&mut self, path: Option<PathBuf>) {
        self.pending_select_path = path;
    }

    pub(super) fn pending_select_path(&self) -> Option<PathBuf> {
        self.pending_select_path.clone()
    }

    pub(super) fn take_pending_select_path(&mut self) -> Option<PathBuf> {
        self.pending_select_path.take()
    }

    pub(super) fn pending_audio(&self) -> Option<PendingAudio> {
        self.pending_audio.clone()
    }

    pub(super) fn set_pending_audio(&mut self, pending: Option<PendingAudio>) {
        self.pending_audio = pending;
    }

    pub(super) fn pending_playback(&self) -> Option<PendingPlayback> {
        self.pending_playback.clone()
    }

    pub(super) fn set_pending_playback(&mut self, pending: Option<PendingPlayback>) {
        self.pending_playback = pending;
    }

    /// Return the in-flight recording waveform refresh request, if any.
    pub(super) fn pending_recording_waveform(&self) -> Option<PendingRecordingWaveform> {
        self.pending_recording_waveform.clone()
    }

    /// Replace the active recording waveform refresh request.
    pub(super) fn set_pending_recording_waveform(
        &mut self,
        pending: Option<PendingRecordingWaveform>,
    ) {
        self.pending_recording_waveform = pending;
    }

    pub(super) fn next_audio_request_id(&mut self) -> u64 {
        let request_id = self.request_counters.next_audio_request_id;
        self.request_counters.next_audio_request_id = self
            .request_counters
            .next_audio_request_id
            .wrapping_add(1)
            .max(1);
        request_id
    }

    /// Generate a request id for recording waveform refresh jobs.
    pub(super) fn next_recording_waveform_request_id(&mut self) -> u64 {
        let request_id = self.request_counters.next_recording_waveform_request_id;
        self.request_counters.next_recording_waveform_request_id = self
            .request_counters
            .next_recording_waveform_request_id
            .wrapping_add(1)
            .max(1);
        request_id
    }

    pub(super) fn send_audio_job(&self, job: AudioLoadJob) -> Result<(), ()> {
        self.audio_loader.publish_latest_request_id(job.request_id);
        self.audio_job_tx.send(job).map_err(|_| ())
    }

    /// Send a background recording waveform refresh job.
    pub(super) fn send_recording_waveform_job(&self, job: RecordingWaveformJob) {
        self.recording_waveform_job_tx.send(job);
    }

    pub(super) fn send_search_job(&self, job: SearchJob) {
        self.search_job_tx.send(job);
    }

    pub(super) fn scan_in_progress(&self) -> bool {
        self.in_progress.scan
    }

    /// Return the source id currently being scanned for folders, if any.
    pub(super) fn pending_folder_scan_source(&self) -> Option<SourceId> {
        self.pending_folder_scan
            .as_ref()
            .map(|pending| pending.source_id.clone())
    }

    /// Start a background scan for folders under `root`, canceling any in-flight scan.
    pub(super) fn request_folder_scan(&mut self, source_id: SourceId, root: PathBuf) -> u64 {
        if let Some(cancel) = self.cancel_handles.folder_scan.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        let request_id = self.request_counters.next_folder_scan_request_id;
        self.request_counters.next_folder_scan_request_id = self
            .request_counters
            .next_folder_scan_request_id
            .wrapping_add(1)
            .max(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_handles.folder_scan = Some(cancel.clone());
        self.pending_folder_scan = Some(PendingFolderScan {
            request_id,
            source_id: source_id.clone(),
        });
        self.spawn_optional_one_shot_job(true, move || {
            let folders = super::library::source_folders::scan_disk_folders(&root, cancel.as_ref());
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            Some(JobMessage::FolderScanFinished(FolderScanResult {
                request_id,
                source_id,
                folders,
            }))
        });
        request_id
    }

    /// Clear folder scan tracking state after a scan completes.
    pub(super) fn clear_folder_scan(&mut self) {
        self.cancel_handles.folder_scan = None;
        self.pending_folder_scan = None;
    }

    /// Return whether a folder scan result matches the latest request.
    pub(super) fn folder_scan_matches(&self, request_id: u64, source_id: &SourceId) -> bool {
        self.pending_folder_scan.as_ref().is_some_and(|pending| {
            pending.request_id == request_id && &pending.source_id == source_id
        })
    }

    pub(super) fn start_scan(&mut self, rx: Receiver<ScanJobMessage>, cancel: Arc<AtomicBool>) {
        self.in_progress.scan = true;
        self.cancel_handles.scan = Some(cancel);
        self.send_source_watch_scan_state(true);
        self.start_progress_stream(rx, JobMessage::Scan, scan_message_is_finished);
    }

    pub(super) fn scan_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.scan.clone()
    }

    pub(super) fn clear_scan(&mut self) {
        self.in_progress.scan = false;
        self.cancel_handles.scan = None;
        self.send_source_watch_scan_state(false);
    }

    fn send_source_watch_scan_state(&self, in_progress: bool) {
        self.source_watcher
            .send(SourceWatchCommand::SetScanInProgress { in_progress });
    }

    /// Forward one stream-based worker channel into the controller job queue.
    fn start_progress_stream<Message: Send + 'static>(
        &self,
        rx: Receiver<Message>,
        wrap: fn(Message) -> JobMessage,
        is_finished: fn(&Message) -> bool,
    ) {
        spawn_progress_forwarder(ProgressForwarderConfig {
            message_tx: self.message_tx.clone(),
            repaint_signal: self.repaint_signal.clone(),
            rx,
            wrap,
            is_finished,
        });
    }

    /// Spawn a one-shot background task that always emits one controller job message.
    fn spawn_one_shot_job<Output: Send + 'static>(
        &self,
        request_repaint: bool,
        run: impl FnOnce() -> Output + Send + 'static,
        wrap: impl FnOnce(Output) -> JobMessage + Send + 'static,
    ) {
        let tx = self.message_tx.clone();
        let signal = self.repaint_signal.clone();
        thread::spawn(move || {
            let _ = tx.send(wrap(run()));
            if request_repaint {
                signal.request_repaint();
            }
        });
    }

    /// Spawn a one-shot background task that may or may not emit a controller job message.
    fn spawn_optional_one_shot_job(
        &self,
        request_repaint: bool,
        run: impl FnOnce() -> Option<JobMessage> + Send + 'static,
    ) {
        let tx = self.message_tx.clone();
        let signal = self.repaint_signal.clone();
        thread::spawn(move || {
            if let Some(message) = run() {
                let _ = tx.send(message);
                if request_repaint {
                    signal.request_repaint();
                }
            }
        });
    }

    pub(super) fn trash_move_in_progress(&self) -> bool {
        self.in_progress.trash_move
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(super) fn start_trash_move(
        &mut self,
        rx: Receiver<trash_move::TrashMoveMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        self.in_progress.trash_move = true;
        self.cancel_handles.trash_move = Some(cancel);
        self.start_progress_stream(rx, JobMessage::TrashMove, trash_move_message_is_finished);
    }

    pub(super) fn trash_move_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.trash_move.clone()
    }

    pub(super) fn clear_trash_move(&mut self) {
        self.in_progress.trash_move = false;
        self.cancel_handles.trash_move = None;
    }

    /// Return whether a background file operation is currently running.
    pub(super) fn file_ops_in_progress(&self) -> bool {
        self.in_progress.file_ops
    }

    /// Begin forwarding file operation progress messages from a background worker.
    pub(super) fn start_file_ops(&mut self, rx: Receiver<FileOpMessage>, cancel: Arc<AtomicBool>) {
        self.in_progress.file_ops = true;
        self.cancel_handles.file_ops = Some(cancel);
        self.start_progress_stream(rx, JobMessage::FileOps, file_op_message_is_finished);
    }

    pub(super) fn file_ops_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.file_ops.clone()
    }

    /// Clear the in-progress state for the current file operation job.
    pub(super) fn clear_file_ops(&mut self) {
        self.in_progress.file_ops = false;
        self.cancel_handles.file_ops = None;
    }

    /// Return whether deferred source DB maintenance is currently running.
    pub(super) fn source_db_maintenance_in_progress(&self) -> bool {
        self.in_progress.source_db_maintenance
    }

    /// Run startup-deferred source DB maintenance in the background.
    pub(super) fn begin_source_db_maintenance(&mut self, jobs: Vec<SourceDbMaintenanceJob>) {
        if self.in_progress.source_db_maintenance || jobs.is_empty() {
            return;
        }
        self.in_progress.source_db_maintenance = true;
        self.spawn_one_shot_job(
            true,
            move || {
                let outcomes = jobs
                    .into_iter()
                    .map(run_source_db_maintenance_job)
                    .collect::<Vec<_>>();
                SourceDbMaintenanceResult { outcomes }
            },
            JobMessage::SourceDbMaintenanceFinished,
        );
    }

    /// Clear the in-progress state for deferred source DB maintenance.
    pub(super) fn clear_source_db_maintenance(&mut self) {
        self.in_progress.source_db_maintenance = false;
    }

    pub(super) fn update_check_in_progress(&self) -> bool {
        self.in_progress.update_check
    }

    /// Return whether an issue-gateway auth polling task is currently running.
    pub(super) fn issue_gateway_poll_in_progress(&self) -> bool {
        self.in_progress.issue_gateway_poll
    }

    pub(super) fn umap_build_in_progress(&self) -> bool {
        self.in_progress.umap_build
    }

    pub(super) fn umap_cluster_build_in_progress(&self) -> bool {
        self.in_progress.umap_cluster_build
    }

    pub(super) fn begin_umap_build(&mut self, job: UmapBuildJob) {
        if self.in_progress.umap_build {
            return;
        }
        self.in_progress.umap_build = true;
        self.spawn_one_shot_job(
            true,
            move || {
                let result = super::ui::map_view::run_umap_build(
                    &job.model_id,
                    &job.umap_version,
                    &job.source_id,
                );
                UmapBuildResult {
                    umap_version: job.umap_version,
                    result,
                }
            },
            JobMessage::UmapBuilt,
        );
    }

    pub(super) fn clear_umap_build(&mut self) {
        self.in_progress.umap_build = false;
    }

    pub(super) fn begin_umap_cluster_build(&mut self, job: UmapClusterBuildJob) {
        if self.in_progress.umap_cluster_build {
            return;
        }
        self.in_progress.umap_cluster_build = true;
        self.spawn_one_shot_job(
            true,
            move || {
                let result = super::ui::map_view::run_umap_cluster_build(
                    &job.model_id,
                    &job.umap_version,
                    job.source_id.as_ref(),
                );
                UmapClusterBuildResult {
                    umap_version: job.umap_version,
                    source_id: job.source_id,
                    result,
                }
            },
            JobMessage::UmapClustersBuilt,
        );
    }

    pub(super) fn clear_umap_cluster_build(&mut self) {
        self.in_progress.umap_cluster_build = false;
    }

    pub(super) fn begin_update_check(&mut self, request: crate::updater::UpdateCheckRequest) {
        if self.in_progress.update_check {
            return;
        }
        self.in_progress.update_check = true;
        self.spawn_one_shot_job(
            true,
            move || UpdateCheckResult {
                result: super::updates::run_update_check(request),
            },
            JobMessage::UpdateChecked,
        );
    }

    pub(super) fn clear_update_check(&mut self) {
        self.in_progress.update_check = false;
    }

    pub(super) fn begin_normalization(&mut self, job: NormalizationJob) {
        self.spawn_one_shot_job(
            true,
            move || run_normalization_job(job),
            JobMessage::Normalized,
        );
    }
}

/// Return whether a scan progress stream item marks terminal completion.
fn scan_message_is_finished(message: &ScanJobMessage) -> bool {
    matches!(message, ScanJobMessage::Finished(_))
}

/// Return whether a trash-move progress stream item marks terminal completion.
fn trash_move_message_is_finished(message: &trash_move::TrashMoveMessage) -> bool {
    matches!(message, trash_move::TrashMoveMessage::Finished(_))
}

/// Return whether a file-op stream item marks terminal completion.
fn file_op_message_is_finished(message: &FileOpMessage) -> bool {
    matches!(message, FileOpMessage::Finished(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_gateway_poll_times_out_after_max_attempts() {
        let cancel = AtomicBool::new(false);
        let mut attempts = 0u32;
        let config = IssueGatewayPollConfig {
            max_attempts: 3,
            max_duration: Duration::from_secs(3600),
            initial_delay: Duration::from_secs(0),
            max_delay: Duration::from_secs(0),
        };

        let result = poll_issue_gateway_with_backoff(
            "request",
            &cancel,
            |_| {
                attempts += 1;
                Ok(None)
            },
            config,
            |_| {},
        );

        match result {
            Some(IssueGatewayAuthResult {
                result: Err(crate::issue_gateway::api::IssueAuthError::TimedOut { attempts, .. }),
            }) => {
                assert_eq!(attempts, 3);
            }
            other => panic!("expected timed out auth result, got {other:?}"),
        }
    }

    #[test]
    fn drops_progress_messages_when_queue_full() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<JobMessage>(1);
        let sender = JobMessageSender::new(tx);
        let _ = sender.send(JobMessage::Scan(ScanJobMessage::Progress {
            completed: 1,
            detail: None,
        }));
        let _ = sender.send(JobMessage::Scan(ScanJobMessage::Progress {
            completed: 2,
            detail: None,
        }));

        let first = rx.try_recv().expect("expected first progress message");
        match first {
            JobMessage::Scan(ScanJobMessage::Progress { completed, .. }) => {
                assert_eq!(completed, 1);
            }
            other => panic!("expected scan progress, got {other:?}"),
        }
        assert!(rx.try_recv().is_err());
    }
}
