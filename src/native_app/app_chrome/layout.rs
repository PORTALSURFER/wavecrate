use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::browser_context_menu;
use crate::native_app::app_chrome::library_browser::library_sidebar;
use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser;
use crate::native_app::app_chrome::metadata_tag_library;
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::overlays;
use crate::native_app::app_chrome::settings::top_control_bar;
use crate::native_app::app_chrome::status_bar::bottom_status_area;
use crate::native_app::app_chrome::toolbar::main_toolbar;
use crate::native_app::app_chrome::view_models::{
    library_sidebar::LibrarySidebarViewModel, sample_browser::SampleBrowserViewModel,
    toolbar::MainToolbarViewModel, waveform_panel::WaveformPanelViewModel,
};
use crate::native_app::app_chrome::waveform_panel::waveform_panel;
use radiant::prelude as ui;

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
    ui::row(
        ui::children()
            .push(library_pane(state))
            .push_if(tag_editor_pane_visible(state), || tag_editor_pane(state))
            .push(sample_workspace(state)),
    )
    .fill()
    .overlays(center_panel_overlays(state))
}

fn center_panel_overlays(state: &NativeAppState) -> ui::Overlays<GuiMessage> {
    ui::overlays()
        .dismissible_context_menu_opt(
            browser_context_menu_overlay(state),
            GuiMessage::CloseContextMenu,
        )
        .blocking_modal_opt(file_move_conflict_overlay(state))
}

fn metadata_completion_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    overlays::metadata_tag_completion(state)
}

fn browser_context_menu_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .ui
        .browser_interaction
        .context_menu
        .as_ref()
        .map(browser_context_menu::overlay)
}

fn file_move_conflict_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .library
        .folder_browser
        .pending_file_move_conflict_view()
        .is_some()
        .then(|| modals::file_move_conflict(state))
}

fn tag_editor_pane_visible(state: &NativeAppState) -> bool {
    state.metadata.tag_library_open && state.library.folder_browser.selected_file_id().is_some()
}

fn tag_editor_pane(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    metadata_tag_library::panel(state)
}

fn library_sidebar(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    library_sidebar::library_sidebar(LibrarySidebarViewModel::from_app_state(state))
}

fn library_pane_overlays(state: &NativeAppState) -> ui::Overlays<GuiMessage> {
    ui::overlays().floating_opt(metadata_completion_overlay(state))
}

fn library_pane(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    let sidebar = library_sidebar(state).overlays(library_pane_overlays(state));
    ui::resizable(sidebar)
        .hover_chrome_only()
        .handle_key("library-sidebar-resize-handle")
        .handle_style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .resize_handle(GuiMessage::ResizeFolder)
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
