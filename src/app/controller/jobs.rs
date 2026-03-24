#![allow(clippy::result_large_err)]

/// Request/state mutation methods for controller background jobs.
mod dispatch;
/// Runtime/build/update dispatch helpers split from the primary dispatch module.
mod dispatch_runtime;
/// File-operation request and result DTOs.
mod file_ops_types;
/// Issue-gateway and issue-token worker runners.
mod issue_gateway_jobs;
/// Controller job lifecycle and queue plumbing.
mod lifecycle;
/// Job message and shared worker DTOs.
mod messages;
/// Background audio normalization worker helpers.
mod normalization_worker;
/// Worker/result forwarding helpers for progress and completion messages.
mod progress_reporting;
/// Bounded job-queue sender/drop policy helpers.
mod queue_orchestration;
/// Retry/backoff policy helpers for async gateway and maintenance jobs.
mod retry_policy;
/// Background selection-export job DTOs and helpers.
mod selection_export_types;
/// Deferred source-db maintenance worker helpers.
mod source_db_maintenance;
/// Internal state and constructor DTOs for controller job orchestration.
mod state;

use self::normalization_worker::run_normalization_job;
use self::progress_reporting::{
    JobForwarderHandles, JobForwarderSpawnConfig, ProgressForwarderConfig, spawn_progress_forwarder,
};
use self::queue_orchestration::new_job_message_queue;
#[cfg(test)]
use self::retry_policy::IssueGatewayPollConfig;
use self::retry_policy::{issue_gateway_poll_config, poll_issue_gateway_with_backoff};
use self::source_db_maintenance::run_source_db_maintenance_job;
use self::state::{JobCancelHandles, JobInProgressState, JobRequestCounters, PendingFolderScan};
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

pub(crate) use self::file_ops_types::*;
pub(crate) use self::messages::*;
pub(crate) use self::queue_orchestration::JobMessageSender;
pub(crate) use self::selection_export_types::*;
pub(super) use self::state::ControllerJobsInit;
pub(crate) use self::state::PendingSliceBatchExport;

type TryRecvError = std::sync::mpsc::TryRecvError;

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
    pub(super) pending_slice_batch_export: Option<PendingSliceBatchExport>,
    request_counters: JobRequestCounters,
    in_progress: JobInProgressState,
    cancel_handles: JobCancelHandles,
    pending_folder_scan: Option<PendingFolderScan>,
    pub(super) repaint_signal: Arc<SharedRepaintSignal>,
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
