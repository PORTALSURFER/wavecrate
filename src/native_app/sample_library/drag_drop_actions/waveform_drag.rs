use std::time::Instant;

use radiant::{
    prelude as ui,
    widgets::{DragHandleMessage, DragHandlePhase},
};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label};
use crate::native_app::waveform::execute_waveform_extraction;

impl NativeAppState {
    pub(in crate::native_app) fn drag_waveform_play_selection(
        &mut self,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
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
                self.library.folder_browser.clear_drag();
                context.end_drag_session();
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
        context: &mut ui::UpdateContext<GuiMessage>,
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
        match self
            .waveform
            .current
            .play_selection_extraction_request(Some(target_folder))
        {
            Ok(request) => {
                self.ui.status.sample = String::from("Preparing dragged range");
                let position = drag.position();
                context
                    .business()
                    .background("gui-waveform-drag-extract")
                    .run(
                        move |_| execute_waveform_extraction(request),
                        move |completion| GuiMessage::PlaySelectionExtractionFinished {
                            completion,
                            drag_position: Some(position),
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
        context: &mut ui::UpdateContext<GuiMessage>,
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
