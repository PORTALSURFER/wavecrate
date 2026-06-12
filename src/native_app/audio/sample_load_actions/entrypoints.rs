use radiant::{prelude as ui, widgets::PointerModifiers};
use std::time::Instant;

use crate::native_app::{
    app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label},
    audio::sample_load_actions::{KEYBOARD_SAMPLE_LOAD_DEBOUNCE, UNCACHED_SAMPLE_LOAD_DEBOUNCE},
};

impl NativeAppState {
    pub(in crate::native_app) fn select_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        self.library
            .folder_browser
            .focus_file_preserving_selection(path.clone());
        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.audio.pending_sample_playback = None;
        self.load_sample(path, context);
    }

    pub(in crate::native_app) fn select_sample_with_modifiers(
        &mut self,
        path: String,
        modifiers: PointerModifiers,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        self.library
            .folder_browser
            .select_file_with_modifiers(path.clone(), modifiers);
        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.audio.pending_sample_playback = None;
        self.load_sample(path, context);
    }

    pub(in crate::native_app) fn load_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.audio.pending_sample_playback = None;
        self.load_sample_with_autoplay(path, context, true);
    }

    pub(in crate::native_app) fn load_sample_without_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.load_sample_with_autoplay(path, context, false);
    }

    fn load_sample_with_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
        autoplay: bool,
    ) {
        let started_at = Instant::now();
        self.cancel_inflight_sample_load();
        if self.start_memory_cached_sample(path.as_str(), autoplay, context, started_at) {
            return;
        }
        if self.start_persisted_cached_sample_load(path.as_str(), autoplay, context, started_at) {
            return;
        }
        self.prepare_uncached_sample_load(path.as_str(), "load_deferred", started_at);
        self.schedule_deferred_sample_load(
            path,
            autoplay,
            false,
            UNCACHED_SAMPLE_LOAD_DEBOUNCE,
            "mouse_or_direct",
            context,
        );
    }

    pub(in crate::native_app) fn defer_navigation_sample_load(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.cancel_inflight_sample_load();
        self.audio.pending_sample_playback = None;
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        if self.start_loaded_navigation_sample(path.as_str(), context, started_at) {
            return;
        }
        if self.start_memory_cached_sample(path.as_str(), true, context, started_at) {
            return;
        }
        self.ui.status.sample = format!("Selected {}", sample_path_label(path.as_str()));
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path.as_str())),
            "navigation_load_deferred",
            started_at,
            None,
        );
        self.schedule_deferred_sample_load(
            path,
            true,
            true,
            KEYBOARD_SAMPLE_LOAD_DEBOUNCE,
            "keyboard",
            context,
        );
    }
}
