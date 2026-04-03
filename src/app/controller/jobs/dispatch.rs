//! Request dispatch/state mutation methods for [`ControllerJobs`].

use super::*;
use std::path::Path;

impl ControllerJobs {
    /// Return true when the requested source still has a pending waveform load.
    pub(in super::super) fn wav_load_pending_for(&self, source_id: &SourceId) -> bool {
        self.pending_source.as_ref() == Some(source_id)
    }

    /// Mark one source as pending waveform load completion.
    pub(in super::super) fn mark_wav_load_pending(&mut self, source_id: SourceId) {
        self.pending_source = Some(source_id);
    }

    /// Clear any pending waveform-load source tracking.
    pub(in super::super) fn clear_wav_load_pending(&mut self) {
        self.pending_source = None;
    }

    /// Queue one waveform load job for the waveform worker thread.
    pub(in super::super) fn send_wav_job(&self, job: WavLoadJob) {
        let _ = self.wav_job_tx.send(job);
    }

    /// Set the optional path to select after async updates complete.
    pub(in super::super) fn set_pending_select_path(&mut self, path: Option<PathBuf>) {
        self.pending_select_path = path;
    }

    /// Return the deferred post-update selection path, if any.
    pub(in super::super) fn pending_select_path(&self) -> Option<PathBuf> {
        self.pending_select_path.clone()
    }

    /// Take and clear the deferred post-update selection path.
    pub(in super::super) fn take_pending_select_path(&mut self) -> Option<PathBuf> {
        self.pending_select_path.take()
    }

    /// Return the in-flight audio load request, if any.
    pub(in super::super) fn pending_audio(&self) -> Option<PendingAudio> {
        self.pending_audio.clone()
    }

    /// Replace the active audio load request.
    pub(in super::super) fn set_pending_audio(&mut self, pending: Option<PendingAudio>) {
        self.pending_audio = pending;
    }

    /// Return the in-flight playback request, if any.
    pub(in super::super) fn pending_playback(&self) -> Option<PendingPlayback> {
        self.pending_playback.clone()
    }

    /// Replace the active playback request.
    pub(in super::super) fn set_pending_playback(&mut self, pending: Option<PendingPlayback>) {
        self.pending_playback = pending;
    }

    /// Return the in-flight recording waveform refresh request, if any.
    pub(in super::super) fn pending_recording_waveform(&self) -> Option<PendingRecordingWaveform> {
        self.pending_recording_waveform.clone()
    }

    /// Replace the active recording waveform refresh request.
    pub(in super::super) fn set_pending_recording_waveform(
        &mut self,
        pending: Option<PendingRecordingWaveform>,
    ) {
        self.pending_recording_waveform = pending;
    }

    /// Return the in-flight slice-batch export request, if any.
    pub(in super::super) fn pending_slice_batch_export(&self) -> Option<PendingSliceBatchExport> {
        self.pending_slice_batch_export.clone()
    }

    /// Replace the active slice-batch export request.
    pub(in super::super) fn set_pending_slice_batch_export(
        &mut self,
        pending: Option<PendingSliceBatchExport>,
    ) {
        self.pending_slice_batch_export = pending;
    }

    /// Return whether the active slice-batch export still matches the provided waveform.
    pub(in super::super) fn pending_slice_batch_export_matches(
        &self,
        request_id: u64,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> bool {
        self.pending_slice_batch_export
            .as_ref()
            .is_some_and(|pending| {
                pending.request_id == request_id
                    && &pending.source_id == source_id
                    && pending.relative_path == relative_path
            })
    }

    /// Clear the active slice-batch export request when the request id still matches.
    pub(in super::super) fn clear_pending_slice_batch_export(&mut self, request_id: u64) {
        if self
            .pending_slice_batch_export
            .as_ref()
            .is_some_and(|pending| pending.request_id == request_id)
        {
            self.pending_slice_batch_export = None;
        }
    }

    /// Generate a request id for audio-load jobs.
    pub(in super::super) fn next_source_hydration_request_id(&mut self) -> u64 {
        let request_id = self.request_counters.next_source_hydration_request_id;
        self.request_counters.next_source_hydration_request_id = self
            .request_counters
            .next_source_hydration_request_id
            .wrapping_add(1)
            .max(1);
        request_id
    }

    /// Generate a request id for audio-load jobs.
    pub(in super::super) fn next_audio_request_id(&mut self) -> u64 {
        let request_id = self.request_counters.next_audio_request_id;
        self.request_counters.next_audio_request_id = self
            .request_counters
            .next_audio_request_id
            .wrapping_add(1)
            .max(1);
        request_id
    }

    /// Generate a request id for recording waveform refresh jobs.
    pub(in super::super) fn next_recording_waveform_request_id(&mut self) -> u64 {
        let request_id = self.request_counters.next_recording_waveform_request_id;
        self.request_counters.next_recording_waveform_request_id = self
            .request_counters
            .next_recording_waveform_request_id
            .wrapping_add(1)
            .max(1);
        request_id
    }

    /// Generate a request id for controller-owned similarity query jobs.
    pub(in super::super) fn next_similarity_request_id(&mut self) -> u64 {
        let request_id = self.request_counters.next_similarity_request_id;
        self.request_counters.next_similarity_request_id = self
            .request_counters
            .next_similarity_request_id
            .wrapping_add(1)
            .max(1);
        request_id
    }

    /// Queue one audio-load job after publishing the latest request id for staleness checks.
    pub(in super::super) fn send_audio_job(&self, job: AudioLoadJob) -> Result<(), ()> {
        self.audio_loader.publish_latest_request_id(job.request_id);
        self.audio_job_tx.send(job).map_err(|_| ())
    }

    /// Send a background recording waveform refresh job.
    pub(in super::super) fn send_recording_waveform_job(&self, job: RecordingWaveformJob) {
        self.recording_waveform_job_tx.send(job);
    }

    /// Queue one sample-browser search job.
    pub(in super::super) fn send_search_job(&self, job: SearchJob) {
        self.search_job_tx.send(job);
    }

    /// Return whether a source scan job is currently running.
    pub(in super::super) fn scan_in_progress(&self) -> bool {
        self.in_progress.scan
    }

    /// Return the source id currently being scanned for folders, if any.
    pub(in super::super) fn pending_folder_scan_source(&self) -> Option<SourceId> {
        self.pending_folder_scan
            .as_ref()
            .map(|pending| pending.source_id.clone())
    }

    /// Start a background scan for folders under `root`, canceling any in-flight scan.
    pub(in super::super) fn request_folder_scan(
        &mut self,
        source_id: SourceId,
        root: PathBuf,
    ) -> u64 {
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
            let folders = crate::app::controller::library::source_folders::scan_disk_folders(
                &root,
                cancel.as_ref(),
            );
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
    pub(in super::super) fn clear_folder_scan(&mut self) {
        self.cancel_handles.folder_scan = None;
        self.pending_folder_scan = None;
    }

    /// Return whether a folder scan result matches the latest request.
    pub(in super::super) fn folder_scan_matches(
        &self,
        request_id: u64,
        source_id: &SourceId,
    ) -> bool {
        self.pending_folder_scan.as_ref().is_some_and(|pending| {
            pending.request_id == request_id && &pending.source_id == source_id
        })
    }

    /// Start forwarding stream updates for a source scan operation.
    pub(in super::super) fn start_scan(
        &mut self,
        rx: Receiver<ScanJobMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        self.in_progress.scan = true;
        self.cancel_handles.scan = Some(cancel);
        self.send_source_watch_scan_state(true);
        self.start_progress_stream(rx, JobMessage::Scan, scan_message_is_finished);
    }

    /// Return the cooperative cancel handle for the active source scan.
    pub(in super::super) fn scan_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.scan.clone()
    }

    /// Clear scan in-progress state and notify the source watcher.
    pub(in super::super) fn clear_scan(&mut self) {
        self.in_progress.scan = false;
        self.cancel_handles.scan = None;
        self.send_source_watch_scan_state(false);
    }

    /// Notify the source watcher when scan state transitions.
    fn send_source_watch_scan_state(&self, in_progress: bool) {
        self.source_watcher
            .send(SourceWatchCommand::SetScanInProgress { in_progress });
    }

    /// Return whether a trash-move job is currently running.
    pub(in super::super) fn trash_move_in_progress(&self) -> bool {
        self.in_progress.trash_move
    }

    /// Begin forwarding trash-move progress from a background worker.
    pub(in super::super) fn start_trash_move(
        &mut self,
        rx: Receiver<trash_move::TrashMoveMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        self.in_progress.trash_move = true;
        self.cancel_handles.trash_move = Some(cancel);
        self.start_progress_stream(rx, JobMessage::TrashMove, trash_move_message_is_finished);
    }

    /// Return the cooperative cancel handle for the active trash move.
    pub(in super::super) fn trash_move_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.trash_move.clone()
    }

    /// Clear the in-progress state for the current trash-move job.
    pub(in super::super) fn clear_trash_move(&mut self) {
        self.in_progress.trash_move = false;
        self.cancel_handles.trash_move = None;
    }

    /// Return whether a background file operation is currently running.
    pub(in super::super) fn file_ops_in_progress(&self) -> bool {
        self.in_progress.file_ops
    }

    /// Begin forwarding file operation progress messages from a background worker.
    pub(in super::super) fn start_file_ops(
        &mut self,
        rx: Receiver<FileOpMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        self.in_progress.file_ops = true;
        self.cancel_handles.file_ops = Some(cancel);
        self.start_progress_stream(rx, JobMessage::FileOps, file_op_message_is_finished);
    }

    /// Return the cooperative cancel handle for the active file operation.
    pub(in super::super) fn file_ops_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.file_ops.clone()
    }

    /// Clear the in-progress state for the current file operation job.
    pub(in super::super) fn clear_file_ops(&mut self) {
        self.in_progress.file_ops = false;
        self.cancel_handles.file_ops = None;
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
