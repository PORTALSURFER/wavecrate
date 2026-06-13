//! Pending source, waveform, and deferred-selection dispatch state.

use super::*;

impl ControllerJobs {
    /// Return true when the requested source still has a pending waveform load.
    pub(in super::super::super) fn wav_load_pending_for(&self, source_id: &SourceId) -> bool {
        self.pending_source.as_ref() == Some(source_id)
    }

    /// Mark one source as pending waveform load completion.
    pub(in super::super::super) fn mark_wav_load_pending(&mut self, source_id: SourceId) {
        self.pending_source = Some(source_id);
    }

    /// Clear any pending waveform-load source tracking.
    pub(in super::super::super) fn clear_wav_load_pending(&mut self) {
        self.pending_source = None;
    }

    /// Queue one waveform load job for the waveform worker thread.
    pub(in super::super::super) fn send_wav_job(&self, job: WavLoadJob) {
        let _ = self.wav_job_tx.send(job);
    }

    /// Set the optional path to select after async updates complete.
    pub(in super::super::super) fn set_pending_select_path(&mut self, path: Option<PathBuf>) {
        self.pending_select_path = path;
    }

    /// Return the deferred post-update selection path, if any.
    pub(in super::super::super) fn pending_select_path(&self) -> Option<PathBuf> {
        self.pending_select_path.clone()
    }

    /// Take and clear the deferred post-update selection path.
    pub(in super::super::super) fn take_pending_select_path(&mut self) -> Option<PathBuf> {
        self.pending_select_path.take()
    }
}
