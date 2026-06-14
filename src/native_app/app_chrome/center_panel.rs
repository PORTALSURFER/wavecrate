use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::browser_context_menu;
use crate::native_app::app_chrome::library_browser::library_sidebar;
use crate::native_app::app_chrome::metadata_tag_library;
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::overlays;
use crate::native_app::app_chrome::sample_workspace;
use crate::native_app::app_chrome::view_models::{
    library_sidebar::LibrarySidebarViewModel, sample_workspace::SampleWorkspaceViewModel,
};

pub(in crate::native_app) fn library_sidebar_region(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    let sidebar = library_sidebar_view(state).overlays(library_pane_overlays(state));
    ui::resizable(sidebar)
        .subtle_resize_handle("library-sidebar-resize-handle", GuiMessage::ResizeFolder)
}

fn library_sidebar_view(state: &NativeAppState) -> ui::View<GuiMessage> {
    let model = LibrarySidebarViewModel::from_app_state(state);
    library_sidebar::library_sidebar(model)
}

pub(in crate::native_app) fn metadata_tag_library_region_visible(state: &NativeAppState) -> bool {
    state.metadata.tag_library_open && state.library.folder_browser.selected_file_id().is_some()
}

pub(in crate::native_app) fn metadata_tag_library_region(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    metadata_tag_library::panel(state)
}

pub(in crate::native_app) fn sample_workspace_region(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    sample_workspace::region(SampleWorkspaceViewModel::from_app_state(state))
}

fn library_pane_overlays(state: &NativeAppState) -> ui::Overlays<GuiMessage> {
    ui::overlays().floating_opt(metadata_completion_overlay(state))
}

fn metadata_completion_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    overlays::metadata_tag_completion(state)
}

pub(in crate::native_app) fn active_workspace_overlays(
    state: &NativeAppState,
) -> ui::Overlays<GuiMessage> {
    ui::overlays()
        .dismissible_context_menu_opt(
            browser_context_menu_overlay(state),
            GuiMessage::CloseContextMenu,
        )
        .blocking_modal_opt(folder_delete_confirmation_overlay(state))
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

fn folder_delete_confirmation_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state
        .ui
        .browser_interaction
        .pending_folder_delete
        .is_some()
        .then(|| modals::folder_delete_confirmation(state))
}
