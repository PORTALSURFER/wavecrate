use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::browser_context_menu;
use crate::native_app::app_chrome::library_browser::folder_sidebar;
use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser;
use crate::native_app::app_chrome::metadata_tag_library;
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::overlays;
use crate::native_app::app_chrome::settings::top_control_bar;
use crate::native_app::app_chrome::status_bar::bottom_status_area;
use crate::native_app::app_chrome::toolbar::main_toolbar;
use crate::native_app::app_chrome::view_models::{
    folder_sidebar::FolderSidebarViewModel, sample_browser::SampleBrowserViewModel,
    toolbar::MainToolbarViewModel, waveform_panel::WaveformPanelViewModel,
};
use crate::native_app::app_chrome::waveform_panel::waveform_panel;
use radiant::prelude as ui;

const LIBRARY_SIDEBAR_RESIZE_HANDLE_HIT_WIDTH: f32 = 5.0;
const LIBRARY_SIDEBAR_RESIZE_HANDLE_INSET: f32 = 1.0;

pub(in crate::native_app) fn shell(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    ui::column([
        top_control_bar(state),
        center_panel(state),
        bottom_status_area(state),
    ])
    .spacing(0.0)
    .fill()
}

fn center_panel(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let browser_context_menu = browser_context_menu_layer(state);
    let file_move_conflict = file_move_conflict_layer(state);
    let metadata_completion = metadata_completion_layer(state);

    let mut sections = vec![library_sidebar(state).transient_layer_opt(metadata_completion)];
    if metadata_tag_library_visible(state) {
        sections.push(metadata_tag_library::panel(state));
    }
    sections.push(library_sidebar_resize_handle());
    sections.push(sample_workspace(state));

    ui::row(sections)
        .fill()
        .transient_layer_opt(browser_context_menu)
        .transient_layer_opt(file_move_conflict)
}

fn metadata_completion_layer(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    overlays::metadata_tag_completion(state).map(|view| ui::Layer::floating(view).pass_through())
}

fn browser_context_menu_layer(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .ui
        .browser_interaction
        .context_menu
        .as_ref()
        .map(browser_context_menu::overlay)
        .map(|view| {
            ui::Layer::context_menu(view).dismiss_on_outside_click(GuiMessage::CloseContextMenu)
        })
}

fn file_move_conflict_layer(state: &NativeAppState) -> Option<ui::Layer<GuiMessage>> {
    state
        .library
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
        .then(|| ui::Layer::modal(modals::file_move_conflict(state)).block_input())
}

fn metadata_tag_library_visible(state: &NativeAppState) -> bool {
    state.metadata.tag_library_open && state.library.folder_browser.selected_file_id().is_some()
}

fn library_sidebar(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    folder_sidebar::folder_sidebar(FolderSidebarViewModel::from_app_state(state))
}

fn library_sidebar_resize_handle() -> ui::View<GuiMessage> {
    ui::drag_handle()
        .hover_chrome_only()
        .mapped(GuiMessage::ResizeFolder)
        .key("library-sidebar-resize-handle")
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .width(LIBRARY_SIDEBAR_RESIZE_HANDLE_HIT_WIDTH)
        .fill_height()
        .padding(LIBRARY_SIDEBAR_RESIZE_HANDLE_INSET)
}

fn sample_workspace(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let toolbar = main_toolbar(MainToolbarViewModel::from_app_state(state));
    let waveform = waveform_panel(WaveformPanelViewModel::from_app_state(state));
    let suppress_sample_hover = state.ui.chrome.folder_panel.is_resizing();
    let sample_browser_model = SampleBrowserViewModel::from_app_state(state, suppress_sample_hover);
    ui::column([toolbar, waveform, sample_browser(sample_browser_model)])
        .padding(4.0)
        .fill()
}
