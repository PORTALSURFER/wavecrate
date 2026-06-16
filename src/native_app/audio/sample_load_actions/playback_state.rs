use crate::native_app::app::GuiMessage;
use crate::native_app::app::NativeAppState;
use radiant::prelude as ui;

impl NativeAppState {
    pub(super) fn clear_sample_loading_state(&mut self) {
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        self.background.sample_load_cancel = None;
        self.background.active_sample_load_key = None;
    }

    pub(in crate::native_app) fn waveform_sample_load_active(&self) -> bool {
        self.background.deferred_sample_load_task.active().is_some()
            || self.active_sample_load_task().is_some()
    }

    pub(in crate::native_app) fn active_sample_load_task(&self) -> Option<ui::TaskTicket> {
        let key = self.background.active_sample_load_key.as_ref()?;
        self.background.sample_load_tasks.active(key)
    }

    pub(in crate::native_app) fn sample_cache_warm_should_yield(&self) -> bool {
        self.waveform_sample_load_active()
            || self.audio.pending_playback_start.is_some()
            || self.audio.early_sample_playback_path.is_some()
            || self.waveform.current.is_playing()
            || self.normalization_work_active()
    }

    pub(in crate::native_app) fn normalization_work_active(&self) -> bool {
        self.background.normalization_progress.is_some()
            || !self.background.normalization_queue.is_empty()
    }

    pub(in crate::native_app) fn yield_sample_cache_warm_for_foreground_load(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.cancel_waveform_cache_warm();
        self.pause_active_folder_cache_warm(context);
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
        self.stop_audio_output_playback();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.early_sample_playback_path = None;
    }

    pub(super) fn cancel_inflight_sample_load(&mut self) {
        self.background.deferred_sample_load_task.cancel();
        if let Some(token) = self.background.sample_load_cancel.take() {
            token.cancel();
        }
        if let Some(key) = self.background.active_sample_load_key.take() {
            self.background.sample_load_tasks.cancel(&key);
        }
        self.waveform.load.selection.cancel();
        if self.audio.early_sample_playback_path.is_some() {
            self.stop_audio_output_playback();
            self.audio.current_playback_span = None;
        }
        self.audio.early_sample_playback_path = None;
    }
}
