use radiant::{
    prelude as ui,
    widgets::{DragHandleMessage, DragHandlePhase},
};

use crate::native_app::app::{FolderBrowserMessage, GuiMessage, NativeAppState};

impl NativeAppState {
    pub(in crate::native_app) fn drag_sample_file(
        &mut self,
        path: String,
        drag: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match drag.phase() {
            DragHandlePhase::Started => {
                self.library
                    .folder_browser
                    .begin_file_drag(path, drag.position());
                self.arm_browser_drag(context);
            }
            DragHandlePhase::Moved => {
                self.library
                    .folder_browser
                    .update_drag_pointer(drag.position());
            }
            DragHandlePhase::Ended => {
                self.library.folder_browser.clear_drag();
                context.end_drag_session();
            }
            DragHandlePhase::Cancelled => {
                self.clear_pending_internal_file_drag_paths();
                self.library.folder_browser.clear_drag();
                context.end_drag_session();
            }
            _ => {}
        }
    }

    pub(in crate::native_app) fn drag_folder(
        &mut self,
        folder_id: String,
        drag: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match drag.phase() {
            DragHandlePhase::Started => {
                self.library
                    .folder_browser
                    .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
                self.arm_browser_drag(context);
            }
            DragHandlePhase::Moved => {
                self.library
                    .folder_browser
                    .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
            }
            DragHandlePhase::Ended => {
                if let Some(target_folder_id) =
                    self.library.folder_browser.hovered_drop_target_folder_id()
                {
                    self.drop_browser_drag_on_folder(target_folder_id, context);
                } else {
                    self.library
                        .folder_browser
                        .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
                    context.end_drag_session();
                }
            }
            DragHandlePhase::Cancelled => {
                self.clear_pending_internal_file_drag_paths();
                self.library.folder_browser.clear_drag();
                context.end_drag_session();
            }
            DragHandlePhase::DoubleActivate => {}
        }
    }
}
