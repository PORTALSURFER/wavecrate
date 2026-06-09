use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::browser_context_menu;
use crate::native_app::app_chrome::library_browser::folder_sidebar;
use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser;
use crate::native_app::app_chrome::metadata_tag_library;
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::overlays;
use crate::native_app::app_chrome::settings::top_control_bar;
use crate::native_app::app_chrome::status_bar;
use crate::native_app::app_chrome::toolbar::main_toolbar;
use crate::native_app::app_chrome::view_models::{
    folder_sidebar::FolderSidebarViewModel, sample_browser::SampleBrowserViewModel,
    toolbar::MainToolbarViewModel, waveform_panel::WaveformPanelViewModel,
};
use crate::native_app::app_chrome::waveform_panel::waveform_panel;
use radiant::prelude as ui;

pub(super) const CENTER_PANEL_PADDING: f32 = 6.0;
const FOLDER_SPLITTER_HIT_WIDTH: f32 = 5.0;
const FOLDER_SPLITTER_INSET: f32 = 1.0;

pub(in crate::native_app) fn shell(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    ui::column([
        top_control_bar(state),
        center_panel(state),
        status_bar::bottom_status_area(state),
    ])
    .spacing(0.0)
    .fill()
}

fn center_panel(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let metadata_completion = overlays::metadata_tag_completion(state, CENTER_PANEL_PADDING)
        .map(|view| ui::Layer::floating(view).pass_through());
    let browser_context_menu = state
        .context_menu
        .as_ref()
        .map(browser_context_menu::overlay)
        .map(|view| {
            ui::Layer::context_menu(view).dismiss_on_outside_click(GuiMessage::CloseContextMenu)
        });
    let file_move_conflict = state
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
        .then(|| ui::Layer::modal(modals::file_move_conflict(state)).block_input());

    let mut children = vec![folder_sidebar_panel(state).transient_layer_opt(metadata_completion)];
    if state.metadata.tag_library_open && state.folder_browser.selected_file_id().is_some() {
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
    .transient_layer_opt(browser_context_menu)
    .transient_layer_opt(file_move_conflict)
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
