use radiant::prelude as ui;
use radiant::widgets::DragHandleMessage;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;

const FILE_COLUMN_DRAG_PREVIEW_MIN_WIDTH: f32 = 64.0;
const FILE_COLUMN_DRAG_PREVIEW_MAX_WIDTH: f32 = 180.0;
const FILE_COLUMN_DRAG_PREVIEW_HEIGHT: f32 = 22.0;
const SOURCE_DRAG_PREVIEW_MIN_WIDTH: f32 = 96.0;
const SOURCE_DRAG_PREVIEW_MAX_WIDTH: f32 = 180.0;
const SOURCE_DRAG_PREVIEW_HEIGHT: f32 = 22.0;

impl NativeAppState {
    pub(super) fn drag_source_row(
        &mut self,
        source_id: String,
        message: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> bool {
        let preview = message.started_position().and_then(|position| {
            self.library
                .folder_browser
                .source_label(&source_id)
                .map(|label| (label.to_owned(), position))
        });
        let reordered = self
            .library
            .folder_browser
            .apply_source_reorder_drag(source_id.clone(), message);
        if let Some((label, position)) = preview
            && self.library.folder_browser.source_reorder_drag_source_id()
                == Some(source_id.as_str())
        {
            context.begin_drag(ui::DragRequest::new(
                ui::DragPreview::text_sized(
                    label,
                    ui::DragPreviewTextSizing::new(SOURCE_DRAG_PREVIEW_HEIGHT)
                        .horizontal_padding(32.0)
                        .min_width(SOURCE_DRAG_PREVIEW_MIN_WIDTH)
                        .max_width(SOURCE_DRAG_PREVIEW_MAX_WIDTH),
                ),
                position,
            ));
        } else if message.is_finished() {
            context.end_drag();
        }
        reordered
    }

    pub(super) fn drag_file_column(
        &mut self,
        column_id: String,
        message: DragHandleMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::DragFileColumn(column_id, message));
        self.sync_file_column_drag_preview(context);
    }

    pub(super) fn cancel_file_column_drag(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.library
            .folder_browser
            .apply_message(FolderBrowserMessage::CancelFileColumnDrag);
        context.end_drag();
    }

    fn sync_file_column_drag_preview(&mut self, context: &mut ui::UiUpdateContext<GuiMessage>) {
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
