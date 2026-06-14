use super::super::super::*;

impl AppController {
    pub(crate) fn handle_audio_load_error(&mut self, pending: PendingAudio, error: AudioLoadError) {
        let source = SampleSource {
            id: pending.source_id.clone(),
            root: pending.root.clone(),
        };
        self.clear_failed_audio_handoff(&pending);
        match error {
            AudioLoadError::Missing(msg) => {
                let _ = self.prune_missing_sample(&source, &pending.relative_path);
                self.show_missing_waveform_notice(&pending.relative_path);
                self.set_status(msg, StatusTone::Warning);
            }
            AudioLoadError::Failed(msg) => {
                self.set_status(msg, StatusTone::Error);
            }
        }
        self.ui.waveform.loading = None;
        self.clear_browser_selection_transition(&pending.source_id, &pending.relative_path);
    }

    fn clear_failed_audio_handoff(&mut self, pending: &PendingAudio) {
        if self
            .runtime
            .jobs
            .pending_playback()
            .as_ref()
            .is_some_and(|pending_play| {
                pending_play.source_id == pending.source_id
                    && pending_play.relative_path == pending.relative_path
            })
        {
            self.runtime.jobs.set_pending_playback(None);
        }
        self.runtime.jobs.set_staged_audio_handoff(None);
    }
}
