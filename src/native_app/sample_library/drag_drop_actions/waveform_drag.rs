use std::time::Instant;

use radiant::{
    prelude as ui,
    widgets::{DragHandleMessage, DragHandlePhase},
};
use wavecrate::sample_sources::HarvestDerivationOperation;

use crate::native_app::app::{
    ExtractedFilePlaybackType, GuiMessage, NativeAppState, emit_gui_action, sample_path_label,
};
use crate::native_app::waveform::{WaveformSelectionKind, execute_waveform_extraction};

const WAVEFORM_SELECTION_DRAG_TASK_NAME: &str = "gui-waveform-selection-drag-extract";

impl NativeAppState {
    pub(in crate::native_app) fn drag_loaded_waveform_sample(
        &mut self,
        drag: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> bool {
        match drag.phase() {
            DragHandlePhase::Started => self.start_loaded_waveform_sample_drag(drag, context),
            DragHandlePhase::Moved => {
                self.library
                    .folder_browser
                    .update_drag_pointer(drag.position());
                true
            }
            DragHandlePhase::Ended => {
                if let Some(target_folder_id) =
                    self.library.folder_browser.hovered_drop_target_folder_id()
                {
                    self.drop_browser_drag_on_folder(target_folder_id, context);
                } else {
                    self.library.folder_browser.clear_drag();
                    context.end_drag_session();
                }
                true
            }
            DragHandlePhase::Cancelled => {
                self.clear_pending_internal_file_drag_paths();
                self.library.folder_browser.clear_drag();
                context.end_drag_session();
                true
            }
            DragHandlePhase::DoubleActivate => false,
        }
    }

    fn start_loaded_waveform_sample_drag(
        &mut self,
        drag: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> bool {
        let started_at = Instant::now();
        if !self.waveform.current.has_loaded_sample() {
            let error = String::from("Load a sample before dragging it");
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "waveform.loaded_sample_drag.start",
                Some("waveform"),
                None,
                "empty",
                started_at,
                Some(&error),
            );
            return false;
        }
        let path = self.waveform.current.path();
        if let Some(error) = self
            .library
            .folder_browser
            .file_change_lock_error(path.as_path(), "Sample move")
        {
            self.flash_protected_source_block_if_error(&error, path.as_path());
            self.ui.status.sample = self.protected_source_status_or_error(&error, path.as_path());
            emit_gui_action(
                "waveform.loaded_sample_drag.start",
                Some("waveform"),
                None,
                "blocked",
                started_at,
                Some(&error),
            );
            return false;
        }
        self.library
            .folder_browser
            .begin_extracted_file_drag(path.clone(), drag.position());
        self.arm_browser_drag(context);
        self.ui.status.sample = format!("Dragging {}", sample_path_label(path.as_path()));
        emit_gui_action(
            "waveform.loaded_sample_drag.start",
            Some("waveform"),
            None,
            "success",
            started_at,
            None,
        );
        true
    }

    pub(in crate::native_app) fn drag_waveform_play_selection(
        &mut self,
        drag: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> bool {
        match drag.phase() {
            DragHandlePhase::Started => self.start_waveform_play_selection_drag(drag, context),
            DragHandlePhase::Moved => {
                self.library
                    .folder_browser
                    .update_drag_pointer(drag.position());
                true
            }
            DragHandlePhase::Ended => {
                if let Some(target_folder_id) =
                    self.library.folder_browser.hovered_drop_target_folder_id()
                {
                    self.drop_browser_drag_on_folder(target_folder_id, context);
                } else {
                    self.library.folder_browser.clear_drag();
                    context.end_drag_session();
                }
                true
            }
            DragHandlePhase::Cancelled => {
                self.clear_pending_internal_file_drag_paths();
                self.library.folder_browser.clear_drag();
                context.end_drag_session();
                true
            }
            _ => false,
        }
    }

    fn start_waveform_play_selection_drag(
        &mut self,
        drag: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> bool {
        let started_at = Instant::now();
        let target_folder = match self.library.folder_browser.selected_folder_path() {
            Some(target_folder) => target_folder,
            None => {
                let error = String::from("Select a folder before dragging a range");
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "waveform.selection_drag.start",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
                return false;
            }
        };
        if let Some(error) = self
            .library
            .folder_browser
            .folder_target_lock_error(&target_folder, "Extraction")
        {
            self.flash_denied_waveform_selection_for_error(
                &error,
                self.waveform.current.play_selection(),
                WaveformSelectionKind::Play,
            );
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "waveform.selection_drag.start",
                Some("waveform"),
                None,
                "blocked",
                started_at,
                Some(&error),
            );
            return false;
        }
        match self
            .waveform
            .current
            .play_selection_extraction_request(Some(target_folder))
        {
            Ok(request) => {
                let selection = request.selection();
                let request = request.with_gain(
                    self.normalized_audition_gain_for_span(selection.start(), selection.end()),
                );
                let request = match self.route_harvest_extraction_request(request) {
                    Ok(request) => request,
                    Err(error) => {
                        self.flash_denied_waveform_selection_for_error(
                            &error,
                            Some(selection),
                            WaveformSelectionKind::Play,
                        );
                        self.ui.status.sample = error.clone();
                        emit_gui_action(
                            "waveform.selection_drag.start",
                            Some("waveform"),
                            None,
                            "blocked",
                            started_at,
                            Some(&error),
                        );
                        return false;
                    }
                };
                let position = drag.position();
                self.ui.status.sample = String::from("Extracting dragged range");
                let playback_type =
                    ExtractedFilePlaybackType::from_loop_active(self.audio.loop_playback);
                context
                    .business()
                    .interactive(WAVEFORM_SELECTION_DRAG_TASK_NAME)
                    .run(
                        move |_| execute_waveform_extraction(request),
                        move |completion| GuiMessage::PlaySelectionExtractionFinished {
                            completion,
                            drag_position: Some(position),
                            playback_type,
                            harvest_operation: HarvestDerivationOperation::Extract,
                            focus_derivative: false,
                            started_at,
                        },
                    );
                true
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "waveform.selection_drag.start",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
                false
            }
        }
    }

    pub(in crate::native_app) fn drop_waveform_play_selection_on_sample_list(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(path) = self.library.folder_browser.extracted_file_drag_path() else {
            return;
        };
        context.end_drag_session();
        self.clear_pending_internal_file_drag_paths();
        self.library.folder_browser.clear_drag();
        self.library.folder_browser.refresh_file_path(&path);
        self.ui.status.sample = format!("Extracted {}", sample_path_label(&path));
    }
}
