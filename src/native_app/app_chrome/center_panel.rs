use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::browser_context_menu;
use crate::native_app::app_chrome::library_browser::library_sidebar;
use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser;
use crate::native_app::app_chrome::metadata_tag_library;
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::overlays;
use crate::native_app::app_chrome::toolbar::main_toolbar;
use crate::native_app::app_chrome::view_models::{
    library_sidebar::LibrarySidebarViewModel, sample_browser::SampleBrowserViewModel,
    toolbar::MainToolbarViewModel, waveform_panel::WaveformPanelViewModel,
};
use crate::native_app::app_chrome::waveform_panel::waveform_panel;

pub(in crate::native_app) fn center_panel(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    center_panel_row(state)
        .fill()
        .overlays(active_center_panel_overlays(state))
}

fn center_panel_row(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    ui::row(center_panel_panes(state))
}

fn center_panel_panes(state: &mut NativeAppState) -> ui::Children<GuiMessage> {
    ui::children()
        .push(library_pane(state))
        .push_if(metadata_tag_library_pane_visible(state), || {
            metadata_tag_library_pane(state)
        })
        .push(sample_workspace_pane(state))
}

fn library_pane(state: &NativeAppState) -> ui::View<GuiMessage> {
    let sidebar = library_sidebar_view(state).overlays(library_pane_overlays(state));
    ui::resizable(sidebar)
        .subtle_resize_handle("library-sidebar-resize-handle", GuiMessage::ResizeFolder)
}

fn library_sidebar_view(state: &NativeAppState) -> ui::View<GuiMessage> {
    let model = LibrarySidebarViewModel::from_app_state(state);
    library_sidebar::library_sidebar(model)
}

fn metadata_tag_library_pane_visible(state: &NativeAppState) -> bool {
    state.metadata.tag_library_open && state.library.folder_browser.selected_file_id().is_some()
}

fn metadata_tag_library_pane(state: &NativeAppState) -> ui::View<GuiMessage> {
    metadata_tag_library::panel(state)
}

fn sample_workspace_pane(state: &mut NativeAppState) -> ui::View<GuiMessage> {
    SampleBrowserViewModel::prepare_visible_sample_window(state);
    let toolbar = main_toolbar(MainToolbarViewModel::from_app_state(state));
    let waveform = waveform_panel(WaveformPanelViewModel::from_app_state(state));
    let sample_browser_model = SampleBrowserViewModel::from_app_state(state);
    ui::column([toolbar, waveform, sample_browser(sample_browser_model)])
        .padding(4.0)
        .fill()
}

fn library_pane_overlays(state: &NativeAppState) -> ui::Overlays<GuiMessage> {
    ui::overlays().floating_opt(metadata_completion_overlay(state))
}

fn metadata_completion_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    overlays::metadata_tag_completion(state)
}

fn active_center_panel_overlays(state: &NativeAppState) -> ui::Overlays<GuiMessage> {
    ui::overlays()
        .dismissible_context_menu_opt(
            browser_context_menu_overlay(state),
            GuiMessage::CloseContextMenu,
        )
        .blocking_modal_opt(file_move_conflict_overlay(state))
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
