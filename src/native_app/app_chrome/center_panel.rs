use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::browser_context_menu;
use crate::native_app::app_chrome::library_browser::library_sidebar;
use crate::native_app::app_chrome::metadata_tag_library;
use crate::native_app::app_chrome::modals;
use crate::native_app::app_chrome::sample_workspace;
use crate::native_app::app_chrome::view_models::{
    library_sidebar::LibrarySidebarViewModel, sample_workspace::SampleWorkspaceViewModel,
};

const LIBRARY_SIDEBAR_PADDING: f32 = 4.0;
const METADATA_PANEL_PADDING: f32 = 6.0;
const BOTTOM_STATUS_BAR_HEIGHT: f32 = 30.0;

pub(in crate::native_app) fn library_sidebar_region(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    ui::resizable(library_sidebar_view(state))
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

pub(in crate::native_app) fn library_sidebar_overlays(
    state: &NativeAppState,
) -> ui::Overlays<GuiMessage> {
    ui::overlays().floating_opt(metadata_completion_overlay(state))
}

fn metadata_completion_overlay(state: &NativeAppState) -> Option<ui::View<GuiMessage>> {
    state.library.folder_browser.selected_file_id()?;
    let completion_options = state.metadata_tag_completion_options();
    if completion_options.is_empty() {
        return None;
    }
    let tag_field_content_width =
        library_sidebar::tag_field_content_width(state.ui.chrome.folder_panel.size());
    let inset_x = LIBRARY_SIDEBAR_PADDING + METADATA_PANEL_PADDING;
    let metadata_panel_height = state.library.folder_browser.metadata_panel_height();
    let inset_y = BOTTOM_STATUS_BAR_HEIGHT
        + LIBRARY_SIDEBAR_PADDING
        + metadata_panel_height
        + library_sidebar::TAG_COMPLETION_POPUP_GAP;
    Some(library_sidebar::tag_completion_overlay(
        completion_options.as_slice(),
        tag_field_content_width,
        inset_x,
        inset_y,
    ))
}

pub(in crate::native_app) fn sample_workspace_overlays(
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
