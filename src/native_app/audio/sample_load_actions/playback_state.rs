use crate::native_app::app::NativeAppState;

impl NativeAppState {
    pub(super) fn clear_sample_loading_state(&mut self) {
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        self.background.sample_load_cancel = None;
    }

    pub(in crate::native_app) fn waveform_sample_load_active(&self) -> bool {
        self.background.deferred_sample_load_task.active().is_some()
            || self.background.sample_load_task.active().is_some()
    }

    pub(in crate::native_app) fn sample_cache_warm_should_yield(&self) -> bool {
        self.waveform_sample_load_active()
            || self.audio.pending_playback_start.is_some()
            || self.audio.early_sample_playback_path.is_some()
            || self.waveform.current.is_playing()
    }

    pub(in crate::native_app) fn waveform_input_blocked_by_sample_load(&self) -> bool {
        self.waveform.load.label.is_some()
            && self.waveform_sample_load_active()
            && !self.library.folder_browser.drag_active()
    }

    pub(super) fn stop_current_sample_playback_for_load(&mut self) {
        if !self.waveform.current.is_playing() && self.audio.early_sample_playback_path.is_none() {
            return;
        }
        if let Some(player) = self.audio.player.as_mut() {
            player.stop();
        }
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.early_sample_playback_path = None;
    }

    pub(super) fn cancel_inflight_sample_load(&mut self) {
        self.background.deferred_sample_load_task.cancel();
        if let Some(token) = self.background.sample_load_cancel.take() {
            token.cancel();
        }
        self.background.sample_load_task.cancel();
        if self.audio.early_sample_playback_path.is_some() {
            if let Some(player) = self.audio.player.as_mut() {
                player.stop();
            }
            self.audio.current_playback_span = None;
        }
        self.audio.early_sample_playback_path = None;
    }
}
