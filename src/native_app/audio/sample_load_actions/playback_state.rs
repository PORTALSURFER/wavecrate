use radiant::prelude as ui;
use std::path::Path;

use crate::native_app::app::GuiMessage;
use crate::native_app::app::NativeAppState;

impl NativeAppState {
    pub(super) fn clear_sample_loading_state(&mut self) {
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        self.background.sample_load_cancel = None;
        self.background.active_sample_load_key = None;
    }

    pub(in crate::native_app) fn waveform_sample_load_active(&self) -> bool {
        self.background
            .sample_load_validation_task
            .active()
            .is_some()
            || self.background.deferred_sample_load_task.active().is_some()
            || self.active_sample_load_task().is_some()
    }

    pub(in crate::native_app) fn active_sample_load_task(&self) -> Option<ui::TaskTicket> {
        let key = self.background.active_sample_load_key.as_ref()?;
        self.background.sample_load_tasks.active(key)
    }

    pub(in crate::native_app) fn sample_cache_warm_should_yield(&self) -> bool {
        self.sample_cache_warm_should_pause_active() || self.waveform.current.is_playing()
    }

    pub(in crate::native_app) fn sample_cache_warm_should_pause_active(&self) -> bool {
        self.waveform_sample_load_active()
            || self.audio.pending_playback_start.is_some()
            || self.audio.sample_playback_session.is_some()
            || self.normalization_work_active()
    }

    pub(in crate::native_app) fn normalization_work_active(&self) -> bool {
        self.background.normalization_progress.is_some()
            || !self.background.normalization_queue.is_empty()
    }

    pub(in crate::native_app) fn sample_load_blocked_by_normalization(&self, path: &str) -> bool {
        if !self.normalization_work_active() {
            return false;
        }
        let path = Path::new(path);
        self.background.normalization_active_paths.contains(path)
            || self
                .background
                .normalization_queue
                .iter()
                .any(|item| item.paths.iter().any(|queued| queued.as_path() == path))
    }

    pub(in crate::native_app) fn yield_sample_cache_warm_for_foreground_load(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.yield_sample_cache_warm_for_user_handoff(context);
    }

    pub(in crate::native_app) fn yield_sample_cache_warm_for_user_handoff(
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
        if !self.waveform.current.is_playing() && self.audio.sample_playback_session.is_none() {
            return;
        }
        self.stop_audio_output_playback();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.clear_sample_playback_session();
    }

    pub(super) fn cancel_inflight_sample_load(&mut self) {
        self.cancel_inflight_sample_load_preserving_early_playback(None);
    }

    pub(super) fn cancel_inflight_sample_load_preserving_early_playback_for(&mut self, path: &str) {
        self.cancel_inflight_sample_load_preserving_early_playback(Some(path));
    }

    fn cancel_inflight_sample_load_preserving_early_playback(
        &mut self,
        preserved_early_path: Option<&str>,
    ) {
        self.background.deferred_sample_load_task.cancel();
        self.cancel_active_sample_load_worker();
        self.waveform.load.selection.cancel();
        let preserve_early_playback = preserved_early_path
            .is_some_and(|path| self.audio.active_sample_playback_matches(path));
        if self.audio.sample_playback_session.is_some() && !preserve_early_playback {
            self.stop_audio_output_playback();
            self.audio.current_playback_span = None;
            self.audio.clear_playback_progress();
            self.audio.clear_sample_playback_session();
        }
        if !preserve_early_playback {
            self.audio.clear_sample_playback_session();
        }
    }

    pub(super) fn cancel_active_sample_load_worker(&mut self) {
        let mut cancelled_worker = false;
        let mut cancelled_resource = false;
        if let Some(token) = self.background.sample_load_cancel.take() {
            token.cancel();
            cancelled_worker = true;
        }
        if let Some(key) = self.background.active_sample_load_key.take() {
            cancelled_resource = self.background.sample_load_tasks.cancel(&key);
        }
        if cancelled_worker || cancelled_resource {
            tracing::debug!(
                target: "wavecrate::debug::sample_load",
                event = "browser.sample_load.worker.cancel_previous",
                cancelled_worker,
                cancelled_resource,
                "Cancelled replaced foreground sample-load work"
            );
        }
    }
}
