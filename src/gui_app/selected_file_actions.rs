use super::{GuiAppState, GuiMessage};
use crate::gui_app::{
    file_actions::sample_path_label, launch::emit_gui_action, waveform::WaveformState,
};
use std::time::Instant;

impl GuiAppState {
    pub(super) fn focus_loaded_file(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self.waveform.has_loaded_sample() {
            self.sample_status = String::from("Load a sample to focus it");
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
        let path = self.waveform.path();
        if self.folder_browser.focus_file_across_sources(&path) {
            if let Some(index) = self.folder_browser.selected_audio_file_index() {
                context.scroll_into_view_snapped(
                    crate::gui_app::SAMPLE_BROWSER_LIST_ID,
                    index as f32 * crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT,
                    crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT,
                    crate::gui_app::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS as f32
                        * crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT,
                    crate::gui_app::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS as f32
                        * crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT,
                    crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT,
                );
            }
            self.sample_status = format!("Focused {}", sample_path_label(&path));
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
            self.sample_status = error.clone();
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

    pub(super) fn delete_selected_item(&mut self) {
        if self.folder_browser.selected_file_id().is_some() {
            self.delete_selected_files();
        } else {
            self.delete_selected_folder();
        }
    }

    fn delete_selected_folder(&mut self) {
        let started_at = Instant::now();
        let target = match self.folder_browser.selected_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.sample_status = error.clone();
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
        match self.folder_browser.delete_selected_folder() {
            Ok(status) => {
                self.sample_status = status;
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    Some(&target.name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    Some(&target.name),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn delete_selected_files(&mut self) {
        let started_at = Instant::now();
        let target = match self.folder_browser.selected_file_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.sample_status = error.clone();
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
        let loaded_path = self.waveform.path();
        let deleting_loaded_sample = target.paths.iter().any(|path| path == &loaded_path);

        match self.folder_browser.delete_selected_files() {
            Ok(status) => {
                if deleting_loaded_sample {
                    if let Some(player) = self.audio_player.as_mut() {
                        player.stop();
                    }
                    self.waveform = WaveformState::empty();
                    self.current_playback_span = None;
                }
                self.sample_status = status;
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    Some(&target.label()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    Some(&target.label()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn extract_playmarked_range(&mut self) {
        let started_at = Instant::now();
        match self.waveform.extract_play_selection_to_sibling() {
            Ok(path) => {
                let label = sample_path_label(&path);
                self.waveform.flash_play_selection();
                self.folder_browser.refresh_file_path(&path);
                self.sample_status = format!("Extracted {label}");
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
                self.sample_status = error.clone();
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
