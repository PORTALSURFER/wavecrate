use std::{fs, path::PathBuf, time::Instant};

use radiant::{
    prelude as ui,
    widgets::{DragHandleMessage, DragHandlePhase},
};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label};

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
        match self.extract_waveform_drag_file() {
            Ok(path) => {
                self.waveform.current.flash_play_selection();
                self.library
                    .folder_browser
                    .begin_extracted_file_drag(path.clone(), drag.position());
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

    fn extract_waveform_drag_file(&mut self) -> Result<PathBuf, String> {
        let target_folder = self
            .library
            .folder_browser
            .selected_folder_path()
            .ok_or_else(|| String::from("Select a folder before dragging a range"))?;
        fs::create_dir_all(&target_folder).map_err(|err| {
            format!(
                "failed to create target folder {}: {err}",
                target_folder.display()
            )
        })?;
        let path = self
            .waveform
            .current
            .extract_play_selection_to_folder(&target_folder)?;
        self.library.folder_browser.refresh_file_path(&path);
        Ok(path)
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
