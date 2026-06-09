use crate::native_app::{
    app::{GuiMessage, NativeAppState},
    app_chrome::library_browser::folder_sidebar,
    sample_library::folder_browser::FileColumnDragFeedback,
};
use radiant::prelude as ui;

const FOLDER_SIDEBAR_PADDING: f32 = 4.0;
const METADATA_PANEL_PADDING: f32 = 6.0;
const BOTTOM_STATUS_BAR_HEIGHT: f32 = 30.0;

pub(in crate::native_app) fn metadata_tag_completion(
    state: &NativeAppState,
) -> Option<ui::View<GuiMessage>> {
    state.library.folder_browser.selected_file_id()?;
    let completion_options = state.metadata_tag_completion_options();
    if completion_options.is_empty() {
        return None;
    }
    let tag_field_content_width =
        folder_sidebar::tag_field_content_width(state.ui.chrome.folder_panel.size());
    let inset_x = FOLDER_SIDEBAR_PADDING + METADATA_PANEL_PADDING;
    let inset_y = BOTTOM_STATUS_BAR_HEIGHT
        + FOLDER_SIDEBAR_PADDING
        + folder_sidebar::metadata_tag_completion_bottom_inset(
            state.library.folder_browser.metadata_panel_height(),
        )
        + folder_sidebar::TAG_COMPLETION_POPUP_GAP;
    Some(folder_sidebar::tag_completion_overlay(
        completion_options.as_slice(),
        tag_field_content_width,
        inset_x,
        inset_y,
    ))
}

pub(in crate::native_app) fn sample_column_drag_preview(
    feedback: &FileColumnDragFeedback,
) -> ui::View<GuiMessage> {
    let size = ui::Vector2::new(feedback.width.clamp(64.0, 180.0), 22.0);
    ui::drag_preview_sized(feedback.label.clone(), feedback.pointer, size)
        .key("sample-column-drag-preview")
}
