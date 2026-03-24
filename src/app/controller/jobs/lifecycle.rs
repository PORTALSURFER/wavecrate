//! Lifecycle and queue-plumbing methods for [`ControllerJobs`].

use super::*;
use crate::app::controller::AppController;

impl ControllerJobs {
    /// Build controller job orchestration from pre-spawned worker channels/handles.
    pub(in super::super) fn new(init: ControllerJobsInit) -> Self {
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
        let source_watcher =
            crate::app::controller::source_watcher::spawn_source_watcher(message_tx.clone());
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
            pending_slice_batch_export: None,
            request_counters: JobRequestCounters::default(),
            in_progress: JobInProgressState::default(),
            cancel_handles: JobCancelHandles::default(),
            pending_folder_scan: None,
            repaint_signal,
        }
    }

    /// Non-blocking receive for one queued worker message.
    pub(in super::super) fn try_recv_message(&self) -> Result<JobMessage, TryRecvError> {
        self.message_rx.try_recv()
    }

    /// Clone the bounded sender used by background workers to emit [`JobMessage`] values.
    pub(in super::super) fn message_sender(&self) -> JobMessageSender {
        self.message_tx.clone()
    }

    /// Install or replace the repaint signal used when async jobs publish updates.
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

    /// Forward one stream-based worker channel into the controller job queue.
    pub(super) fn start_progress_stream<Message: Send + 'static>(
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
    pub(super) fn spawn_one_shot_job<Output: Send + 'static>(
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
    pub(super) fn spawn_optional_one_shot_job(
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
}

impl AppController {
    /// Install the repaint signal used by controller-owned async subsystems.
    pub(crate) fn set_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.runtime.jobs.set_repaint_signal(signal.clone());
        self.runtime.analysis.set_repaint_signal(signal);
    }

    /// Shut down background workers owned by the controller.
    pub(crate) fn shutdown(&mut self) {
        self.runtime.jobs.shutdown();
        self.runtime.analysis.shutdown();
    }
}
