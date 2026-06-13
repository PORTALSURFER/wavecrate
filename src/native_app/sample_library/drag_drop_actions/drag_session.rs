use radiant::prelude as ui;

use crate::native_app::{
    app::{GuiMessage, NativeAppState},
    sample_library::{DRAG_PREVIEW_HEIGHT, DRAG_PREVIEW_MAX_WIDTH},
};

impl NativeAppState {
    pub(in crate::native_app) fn arm_browser_drag(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let drag = self.library.folder_browser.drag_preview().map(|preview| {
            ui::DragRequest::new(
                ui::DragPreview::text_sized(
                    preview.label,
                    ui::DragPreviewTextSizing::new(DRAG_PREVIEW_HEIGHT)
                        .min_width(96.0)
                        .max_width(DRAG_PREVIEW_MAX_WIDTH),
                ),
                preview.pointer,
            )
        });
        let external = self.library.folder_browser.external_drag_request();
        self.arm_pending_internal_file_drag_paths(external.as_ref());

        context.begin_drag_session(drag, external, GuiMessage::ExternalDragCompleted);
    }

    pub(in crate::native_app) fn cancel_browser_drag_on_sample_list(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.library.folder_browser.clear_drag();
        self.clear_pending_internal_file_drag_paths();
        context.end_drag_session();
        self.ui.status.sample = String::from("Drag cancelled");
    }
}
