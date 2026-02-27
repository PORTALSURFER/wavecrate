//! Coalesced latest-only queue and worker lifecycle for recording waveform jobs.

use super::*;

#[derive(Default)]
pub(super) struct RecordingWaveformJobQueueState {
    pending: Option<RecordingWaveformJob>,
    shutdown: bool,
}

/// Latest-only queue for recording waveform refresh jobs.
pub(super) struct RecordingWaveformJobQueue {
    pub(super) state: Mutex<RecordingWaveformJobQueueState>,
    ready: Condvar,
}

impl RecordingWaveformJobQueue {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(RecordingWaveformJobQueueState::default()),
            ready: Condvar::new(),
        }
    }

    pub(super) fn send(&self, job: RecordingWaveformJob) {
        let mut state = self.lock_state();
        if state.shutdown {
            return;
        }
        state.pending = Some(job);
        self.ready.notify_one();
    }

    pub(super) fn shutdown(&self) {
        let mut state = self.lock_state();
        state.shutdown = true;
        state.pending = None;
        self.ready.notify_all();
    }

    pub(super) fn take_blocking(&self) -> Option<RecordingWaveformJob> {
        let mut state = self.lock_state();
        loop {
            if state.shutdown {
                return None;
            }
            if let Some(job) = state.pending.take() {
                return Some(job);
            }
            state = self.wait_ready(state);
        }
    }

    #[cfg(test)]
    pub(super) fn try_take(&self) -> Option<RecordingWaveformJob> {
        let mut state = self.lock_state();
        state.pending.take()
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, RecordingWaveformJobQueueState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Recording waveform queue lock poisoned; recovering.");
                poisoned.into_inner()
            }
        }
    }

    fn wait_ready<'a>(
        &self,
        guard: std::sync::MutexGuard<'a, RecordingWaveformJobQueueState>,
    ) -> std::sync::MutexGuard<'a, RecordingWaveformJobQueueState> {
        self.ready.wait(guard).unwrap_or_else(|poisoned| {
            warn!("Recording waveform queue condvar poisoned; recovering.");
            poisoned.into_inner()
        })
    }
}

/// Sender handle for coalesced recording waveform refresh requests.
#[derive(Clone)]
pub(crate) struct RecordingWaveformJobSender {
    queue: Arc<RecordingWaveformJobQueue>,
}

impl RecordingWaveformJobSender {
    /// Replace any pending recording waveform job with the latest request.
    pub(crate) fn send(&self, job: RecordingWaveformJob) {
        self.queue.send(job);
    }
}

/// Join handle and shutdown signal for the recording waveform worker thread.
pub(crate) struct RecordingWaveformWorkerHandle {
    queue: Arc<RecordingWaveformJobQueue>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl RecordingWaveformWorkerHandle {
    /// Signal the worker thread to exit and wait for it to finish.
    pub(crate) fn shutdown(&mut self) {
        self.queue.shutdown();
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn a background worker that processes the latest pending recording waveform job.
/// Returns the sender, result channel, and a shutdown handle.
pub(crate) fn spawn_recording_waveform_loader() -> (
    RecordingWaveformJobSender,
    Receiver<RecordingWaveformLoadResult>,
    RecordingWaveformWorkerHandle,
) {
    let queue = Arc::new(RecordingWaveformJobQueue::new());
    let sender = RecordingWaveformJobSender {
        queue: Arc::clone(&queue),
    };
    let (result_tx, result_rx) = std::sync::mpsc::channel::<RecordingWaveformLoadResult>();
    let queue_worker = Arc::clone(&queue);
    let handle = thread::spawn(move || {
        while let Some(job) = queue_worker.take_blocking() {
            let result = load_recording_waveform(job);
            let _ = result_tx.send(result);
        }
    });
    (
        sender,
        result_rx,
        RecordingWaveformWorkerHandle {
            queue,
            join_handle: Some(handle),
        },
    )
}
