use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::library_browser::folder_sidebar::{
    self, FolderSidebarViewModel,
};
use crate::native_app::app_chrome::library_browser::sample_browser_view::{
    SampleBrowserViewModel, sample_browser,
};
use crate::native_app::app_chrome::status_bar;
use crate::native_app::app_chrome::toolbar::{MainToolbarViewModel, main_toolbar};
use crate::native_app::app_chrome::waveform_panel::{WaveformPanelViewModel, waveform_panel};
use crate::native_app::app_chrome::{context_menu_overlay, metadata_tag_library, modals, overlays};
use crate::native_app::audio::audio_settings::top_status_bar;
use radiant::prelude as ui;

const CENTER_PANEL_PADDING: f32 = 6.0;
const FOLDER_SPLITTER_HIT_WIDTH: f32 = 5.0;
const FOLDER_SPLITTER_INSET: f32 = 1.0;

pub(in crate::native_app) fn view(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let tag_completion_overlay = overlays::metadata_tag_completion(state, CENTER_PANEL_PADDING);
    let content = ui::column([
        top_status_bar(state),
        center_panel(state),
        status_bar::bottom_status_bar(status_bar::StatusBarViewModel::from_app_state(state)),
    ])
    .spacing(0.0)
    .fill();
    let mut layers = vec![content];
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
        layers.push(context_menu_overlay::overlay(menu));
    }
    if let Some(feedback) = state.folder_browser.file_column_drag_feedback() {
        layers.push(overlays::sample_column_drag_preview(&feedback));
    }
    ui::stack_layers(layers).fill()
}

fn center_panel(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let mut children = vec![folder_sidebar_panel(state)];
    if state.metadata_tag_library_open && state.folder_browser.selected_file_id().is_some() {
        children.push(metadata_tag_library::panel(state));
    }
    children.push(folder_splitter());
    children.push(main_area(state));
    ui::column([
        ui::spacer().height(CENTER_PANEL_PADDING).fill_width(),
        ui::row(children).padding_x(CENTER_PANEL_PADDING).fill(),
    ])
    .spacing(0.0)
    .fill()
}

fn folder_sidebar_panel(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    folder_sidebar::folder_sidebar(FolderSidebarViewModel::from_app_state(state))
}

fn folder_splitter() -> ui::View<GuiMessage> {
    ui::drag_handle()
        .hover_chrome_only()
        .mapped(GuiMessage::ResizeFolder)
        .key("folder-browser-splitter-handle")
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .width(FOLDER_SPLITTER_HIT_WIDTH)
        .fill_height()
        .padding(FOLDER_SPLITTER_INSET)
}

fn main_area(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let toolbar = main_toolbar(MainToolbarViewModel::from_app_state(state));
    let waveform = waveform_panel(WaveformPanelViewModel::from_app_state(state));
    let suppress_sample_hover = state.folder_panel.is_resizing();
    let sample_browser_model = SampleBrowserViewModel::from_app_state(state, suppress_sample_hover);
    ui::column([toolbar, waveform, sample_browser(sample_browser_model)])
        .padding(4.0)
        .fill()
}
