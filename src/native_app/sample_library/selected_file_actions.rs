use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app::{emit_gui_action, sample_path_label};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT, SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
};
use crate::native_app::waveform::{WaveformExtractionCompletion, execute_waveform_extraction};
use radiant::gui::types::Point;
use std::time::Instant;

impl NativeAppState {
    pub(in crate::native_app) fn focus_loaded_file(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
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
                    SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                    SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
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

    pub(in crate::native_app) fn delete_selected_item(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        if self.library.folder_browser.selected_file_id().is_some() {
            self.delete_selected_files(context);
        } else {
            self.delete_selected_folder(context);
        }
    }

    fn delete_selected_folder(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
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
        self.move_selected_folder_to_trash(target.path, started_at, context);
    }

    fn delete_selected_files(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
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
        self.move_selected_files_to_trash(target.paths, started_at, context);
    }

    pub(in crate::native_app) fn extract_playmarked_range(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self
            .waveform
            .current
            .play_selection_extraction_request(None)
        {
            Ok(request) => {
                if let Some(error) = self
                    .library
                    .folder_browser
                    .file_change_lock_error(request.source_path(), "Extraction")
                {
                    self.ui.status.sample = error.clone();
                    emit_gui_action(
                        "waveform.extract_playmarked_range",
                        Some("waveform"),
                        None,
                        "blocked",
                        started_at,
                        Some(&error),
                    );
                    return;
                }
                self.ui.status.sample = String::from("Extracting play range");
                context.business().background("gui-waveform-extract").run(
                    move |_| execute_waveform_extraction(request),
                    move |completion| GuiMessage::PlaySelectionExtractionFinished {
                        completion,
                        drag_position: None,
                        started_at,
                    },
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

    pub(in crate::native_app) fn finish_play_selection_extraction(
        &mut self,
        completion: WaveformExtractionCompletion,
        drag_position: Option<Point>,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        match completion.result {
            Ok(path) => {
                self.waveform
                    .current
                    .mark_extracted_play_selection(&completion.source_path, completion.selection);
                self.waveform.current.flash_play_selection();
                self.library.folder_browser.refresh_file_path(&path);
                if let Some(position) = drag_position {
                    self.library
                        .folder_browser
                        .begin_extracted_file_drag(path.clone(), position);
                    self.arm_browser_drag(context);
                    self.ui.status.sample = format!("Dragging {}", sample_path_label(&path));
                    emit_gui_action(
                        "waveform.selection_drag.start",
                        Some("waveform"),
                        None,
                        "success",
                        started_at,
                        None,
                    );
                } else {
                    let label = sample_path_label(&path);
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
            }
            Err(error) => {
                let action = if drag_position.is_some() {
                    "waveform.selection_drag.start"
                } else {
                    "waveform.extract_playmarked_range"
                };
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    action,
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
