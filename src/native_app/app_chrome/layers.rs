use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::{browser_context_menu, layout, modals, overlays, status_bar};
use radiant::prelude as ui;

const PLAYBACK_CURSOR_OVERLAY_KEY: u64 = 0x706c_6179_6375_7273;
const PLAYBACK_CURSOR_OVERLAY_FPS: u32 = 60;

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
        .frame_clock(frame_clock())
        .overlay(playback_cursor_overlay())
        .into_view()
        .fill()
}

fn frame_clock() -> ui::FrameClock<NativeAppState, GuiMessage> {
    ui::FrameClock::message(GuiMessage::Frame)
        .when(|state: &mut NativeAppState| state.frame_message_animation_active())
        .repaint_scope(
            |state: &mut NativeAppState| state.frame_repaint_scope_before_update(),
            |state, scope| state.frame_can_use_paint_only(scope),
        )
}

fn playback_cursor_overlay() -> ui::TransientOverlay<NativeAppState> {
    ui::TransientOverlay::new(PLAYBACK_CURSOR_OVERLAY_KEY)
        .paint_only()
        .when(|state: &mut NativeAppState| state.waveform.is_playing())
        .fps(PLAYBACK_CURSOR_OVERLAY_FPS)
        .paint(NativeAppState::paint_playback_overlay)
}

fn metadata_tag_completion_layer(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    overlays::metadata_tag_completion(state, layout::CENTER_PANEL_PADDING)
        .map(|view| ui::Layer::floating(view).pass_through())
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
        .then(|| ui::Layer::modal(modals::transaction_list(state)).block_input())
}

fn file_move_conflict_modal(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
        .then(|| ui::Layer::modal(modals::file_move_conflict(state)).block_input())
}

fn browser_context_menu_layer(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .context_menu
        .as_ref()
        .map(browser_context_menu::overlay)
        .map(|view| {
            ui::Layer::context_menu(view).dismiss_on_outside_click(GuiMessage::CloseContextMenu)
        })
}

fn sample_column_drag_preview(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .folder_browser
        .file_column_drag_feedback()
        .map(|feedback| overlays::sample_column_drag_preview(&feedback))
        .map(|view| ui::Layer::drag_preview(view).pass_through())
}
