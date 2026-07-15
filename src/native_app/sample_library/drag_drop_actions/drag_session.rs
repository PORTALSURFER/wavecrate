use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::{
    app::{GuiMessage, NativeAppState},
    sample_library::{DRAG_PREVIEW_HEIGHT, DRAG_PREVIEW_MAX_WIDTH},
};

impl NativeAppState {
    pub(in crate::native_app) fn arm_browser_drag(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.arm_browser_drag_with_handoff_rating(context, true);
    }

    pub(in crate::native_app) fn arm_browser_drag_without_handoff_rating(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.arm_browser_drag_with_handoff_rating(context, false);
    }

    fn arm_browser_drag_with_handoff_rating(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        add_keep_rating: bool,
    ) {
        let started_at = Instant::now();
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
        let path_count = external
            .as_ref()
            .map(|request| match &request.payload {
                radiant::runtime::ExternalDragPayload::Files(paths) => paths.len(),
            })
            .unwrap_or_default();
        tracing::debug!(
            target: "wavecrate::external_drag",
            event = "external_drag.payload_ready",
            path_count,
            elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0,
            "External drag payload ready"
        );
        self.arm_pending_internal_file_drag_paths(external.as_ref(), add_keep_rating);

        context.begin_drag_session(drag, external, GuiMessage::ExternalDragCompleted);
        tracing::debug!(
            target: "wavecrate::external_drag",
            event = "external_drag.armed",
            path_count,
            elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0,
            "External drag armed"
        );
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
