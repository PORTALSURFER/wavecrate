use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::{browser_context_menu, layout, modals, overlays, status_bar};
use radiant::prelude as ui;

pub(in crate::native_app) fn view(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let tag_completion_overlay =
        overlays::metadata_tag_completion(state, layout::CENTER_PANEL_PADDING);
    let mut layers = vec![layout::shell(state)];

    if state.job_details_open
        && let Some(progress) = state.folder_progress.as_ref()
    {
        layers.push(status_bar::job_details_popover(progress));
    }
    if state.transaction_list_open {
        layers.push(modals::transaction_list(state));
    }
    if state
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
    {
        layers.push(modals::file_move_conflict(state));
    }
    if let Some(overlay) = tag_completion_overlay {
        layers.push(overlay);
    }
    if let Some(menu) = state.context_menu.as_ref() {
        layers.push(browser_context_menu::overlay(menu));
    }
    if let Some(feedback) = state.folder_browser.file_column_drag_feedback() {
        layers.push(overlays::sample_column_drag_preview(&feedback));
    }

    ui::stack_layers(layers).fill()
}
