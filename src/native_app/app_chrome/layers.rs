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

    ui::scene(layout::shell(state))
        .layer_opt(metadata_completion)
        .layer_opt(job_details)
        .layer_opt(transaction_list)
        .layer_opt(file_move_conflict)
        .layer_opt(browser_context_menu)
        .layer_opt(sample_drag_preview)
        .into_view()
        .fill()
}

fn metadata_tag_completion_layer(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    overlays::metadata_tag_completion(state, layout::CENTER_PANEL_PADDING).map(ui::Layer::floating)
}

fn job_details_popover(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    if state.job_details_open
        && let Some(progress) = state.folder_progress.as_ref()
    {
        return Some(ui::Layer::popover(status_bar::job_details_popover(
            progress,
        )));
    }

    None
}

fn transaction_list_modal(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .transaction_list_open
        .then(|| ui::Layer::modal(modals::transaction_list(state)))
}

fn file_move_conflict_modal(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
        .then(|| ui::Layer::modal(modals::file_move_conflict(state)))
}

fn browser_context_menu_layer(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .context_menu
        .as_ref()
        .map(browser_context_menu::overlay)
        .map(ui::Layer::context_menu)
}

fn sample_column_drag_preview(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .folder_browser
        .file_column_drag_feedback()
        .map(|feedback| overlays::sample_column_drag_preview(&feedback))
        .map(ui::Layer::drag_preview)
}
