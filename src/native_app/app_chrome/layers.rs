use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::{browser_context_menu, layout, modals, overlays, status_bar};
use radiant::prelude as ui;

pub(in crate::native_app) fn view(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let metadata_completion = metadata_tag_completion_layer(state);
    let job_details = job_details_popover(state);
    let transaction_list = transaction_list_modal(state);
    let file_move_conflict = file_move_conflict_modal(state);
    let browser_context_menu = browser_context_menu_layer(state);
    let sample_drag_preview = sample_column_drag_preview(state);

    ui::layer_host(layout::shell(state))
        .floating_opt(metadata_completion)
        .popover_opt(job_details)
        .modal_opt(transaction_list)
        .modal_opt(file_move_conflict)
        .context_menu_opt(browser_context_menu)
        .drag_preview_opt(sample_drag_preview)
        .into_view()
        .fill()
}

fn metadata_tag_completion_layer(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    overlays::metadata_tag_completion(state, layout::CENTER_PANEL_PADDING)
}

fn job_details_popover(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    if state.job_details_open
        && let Some(progress) = state.folder_progress.as_ref()
    {
        return Some(status_bar::job_details_popover(progress));
    }

    None
}

fn transaction_list_modal(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .transaction_list_open
        .then(|| modals::transaction_list(state))
}

fn file_move_conflict_modal(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
        .then(|| modals::file_move_conflict(state))
}

fn browser_context_menu_layer(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .context_menu
        .as_ref()
        .map(browser_context_menu::overlay)
}

fn sample_column_drag_preview(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .folder_browser
        .file_column_drag_feedback()
        .map(|feedback| overlays::sample_column_drag_preview(&feedback))
}
