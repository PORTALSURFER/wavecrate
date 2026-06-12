use radiant::prelude as ui;
use radiant::widgets::DragHandleMessage;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;

const FILE_COLUMN_DRAG_PREVIEW_MIN_WIDTH: f32 = 64.0;
const FILE_COLUMN_DRAG_PREVIEW_MAX_WIDTH: f32 = 180.0;
const FILE_COLUMN_DRAG_PREVIEW_HEIGHT: f32 = 22.0;

impl NativeAppState {
    pub(super) fn drag_file_column(
        &mut self,
        column_id: String,
        message: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::DragFileColumn(column_id, message));
        self.sync_file_column_drag_preview(context);
    }

    pub(super) fn cancel_file_column_drag(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::CancelFileColumnDrag);
        context.end_drag();
    }

    fn sync_file_column_drag_preview(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let Some(feedback) = self.library.folder_browser.file_column_drag_feedback() else {
            context.end_drag();
            return;
        };
        let size = ui::Vector2::new(
            feedback.width.clamp(
                FILE_COLUMN_DRAG_PREVIEW_MIN_WIDTH,
                FILE_COLUMN_DRAG_PREVIEW_MAX_WIDTH,
            ),
            FILE_COLUMN_DRAG_PREVIEW_HEIGHT,
        );
        context.begin_drag(ui::DragRequest::new(
            ui::DragPreview::sized(feedback.label, size),
            feedback.pointer,
        ));
    }
}
