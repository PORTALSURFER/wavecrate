use radiant::{prelude as ui, widgets::PointerModifiers};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label};

impl NativeAppState {
    pub(in crate::native_app) fn select_sample(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.prune_missing_sample_before_load(path.as_str(), started_at) {
            return;
        }
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
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.prune_missing_sample_before_load(path.as_str(), started_at) {
            return;
        }
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
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.audio.pending_sample_playback = None;
        self.load_sample_with_autoplay(path, context, true);
    }

    pub(in crate::native_app) fn load_sample_without_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.prune_missing_sample_before_load(path.as_str(), started_at) {
            return;
        }
        self.load_sample_with_autoplay(path, context, false);
    }

    fn load_sample_with_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        autoplay: bool,
    ) {
        let started_at = Instant::now();
        if self.prune_missing_sample_before_load(path.as_str(), started_at) {
            return;
        }
        self.yield_sample_cache_warm_for_foreground_load(context);
        self.cancel_inflight_sample_load();
        if self.start_memory_cached_sample(path.as_str(), autoplay, context, started_at) {
            return;
        }
        self.start_foreground_sample_load(path.as_str(), autoplay, context, started_at);
    }

    pub(in crate::native_app) fn load_navigation_sample(
        &mut self,
        path: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.prune_missing_sample_before_load(path.as_str(), started_at) {
            return;
        }
        self.yield_sample_cache_warm_for_foreground_load(context);
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
        self.start_foreground_sample_load(path.as_str(), true, context, started_at);
    }

    fn prune_missing_sample_before_load(&mut self, path: &str, started_at: Instant) -> bool {
        if sample_path_is_existing_file(Path::new(path)) {
            return false;
        }

        let absolute_path = PathBuf::from(path);
        let Some((source, relative_path)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&absolute_path)
        else {
            return false;
        };
        let changed = self
            .library
            .folder_browser
            .refresh_filesystem_paths(source.id.as_str(), &[relative_path]);
        if !changed {
            return false;
        }

        self.audio.pending_sample_playback = None;
        self.ui.status.sample = format!("Removed missing {}", sample_path_label(path));
        if let Err(error) = self.library.folder_browser.save_source_scan_cache() {
            self.ui.status.sample =
                format!("{}; source cache not saved: {error}", self.ui.status.sample);
            emit_gui_action(
                "browser.select_sample.source_cache_persist",
                Some("browser"),
                Some(&sample_path_label(path)),
                "error",
                started_at,
                Some(&error),
            );
        }
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path)),
            "missing_pruned",
            started_at,
            None,
        );
        true
    }
}

fn sample_path_is_existing_file(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.is_file(),
        Err(err) if err.kind() == ErrorKind::NotFound => false,
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "Could not verify selected sample path before load"
            );
            true
        }
    }
}
