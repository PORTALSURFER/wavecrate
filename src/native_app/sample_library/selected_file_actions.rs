use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app::{emit_gui_action, sample_path_label};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT,
};
use std::time::Instant;

impl NativeAppState {
    pub(in crate::native_app) fn focus_loaded_file(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self.waveform.current.has_loaded_sample() {
            self.ui.status.sample = String::from("Load a sample to focus it");
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        }
        let path = self.waveform.current.path();
        if self.library.folder_browser.focus_file_across_sources(&path) {
            if let Some(index) = self
                .library
                .folder_browser
                .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
            {
                context.scroll_into_view_snapped(
                    SAMPLE_BROWSER_LIST_ID,
                    index as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_ROW_HEIGHT,
                );
            }
            self.ui.status.sample = format!("Focused {}", sample_path_label(&path));
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "success",
                started_at,
                None,
            );
        } else {
            let error = format!(
                "Loaded sample is not visible in sources: {}",
                path.display()
            );
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "not_found",
                started_at,
                Some(&error),
            );
        }
    }

    pub(in crate::native_app) fn delete_selected_item(&mut self) {
        if self.library.folder_browser.selected_file_id().is_some() {
            self.delete_selected_files();
        } else {
            self.delete_selected_folder();
        }
    }

    fn delete_selected_folder(&mut self) {
        let started_at = Instant::now();
        let target = match self.library.folder_browser.selected_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        self.move_selected_folder_to_trash(target.path, started_at);
    }

    fn delete_selected_files(&mut self) {
        let started_at = Instant::now();
        let target = match self.library.folder_browser.selected_file_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        self.move_selected_files_to_trash(target.paths, started_at);
    }

    pub(in crate::native_app) fn extract_playmarked_range(&mut self) {
        let started_at = Instant::now();
        match self.waveform.current.extract_play_selection_to_sibling() {
            Ok(path) => {
                let label = sample_path_label(&path);
                self.waveform.current.flash_play_selection();
                self.library.folder_browser.refresh_file_path(&path);
                self.ui.status.sample = format!("Extracted {label}");
                emit_gui_action(
                    "waveform.extract_playmarked_range",
                    Some("waveform"),
                    Some(&label),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "waveform.extract_playmarked_range",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}
