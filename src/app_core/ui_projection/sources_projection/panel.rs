use super::FolderPaneId;
use super::folder_panes::{project_folder_pane, project_folder_pane_id};
use super::source_rows::{
    project_loading_source_row, project_mutation_busy_source_row, project_source_rows,
};
use crate::app_core::actions::{
    NativeFolderPaneIdModel as FolderPaneIdModel, NativeSourcesPanelModel as SourcesPanelModel,
};
use crate::app_core::controller::AppController;

/// Project source/folder panel data for the native sidebar.
pub(crate) fn project_sources_model(controller: &AppController) -> SourcesPanelModel {
    let ui = &controller.ui;
    let upper_folder_pane = project_folder_pane(controller, FolderPaneId::Upper);
    let lower_folder_pane = project_folder_pane(controller, FolderPaneId::Lower);
    let active_folder_pane = project_folder_pane_id(ui.sources.active_folder_pane);
    let active_pane_model = match active_folder_pane {
        FolderPaneIdModel::Upper => &upper_folder_pane,
        FolderPaneIdModel::Lower => &lower_folder_pane,
    };
    let active_tree_search_query = active_pane_model.tree_search_query.clone();
    let active_show_all_items = active_pane_model.show_all_items;
    let active_can_toggle_show_all_items = active_pane_model.can_toggle_show_all_items;
    let active_flattened_view = active_pane_model.flattened_view;
    let active_can_toggle_flattened_view = active_pane_model.can_toggle_flattened_view;
    let active_focused_tree_row = active_pane_model.focused_tree_row;
    let active_tree_rows = active_pane_model.tree_rows.clone();
    let active_tree_actions = active_pane_model.tree_actions.clone();
    let active_recovery = active_pane_model.recovery.clone();

    SourcesPanelModel {
        header: format!("Library ({} items)", ui.browser.viewport.visible.len()),
        search_query: active_tree_search_query.clone(),
        active_folder_pane,
        upper_folder_pane,
        lower_folder_pane,
        tree_search_query: active_tree_search_query,
        show_all_items: active_show_all_items,
        can_toggle_show_all_items: active_can_toggle_show_all_items,
        flattened_view: active_flattened_view,
        can_toggle_flattened_view: active_can_toggle_flattened_view,
        selected_row: ui.sources.selected,
        loading_row: project_loading_source_row(ui),
        mutation_busy_row: project_mutation_busy_source_row(controller),
        focused_tree_row: active_focused_tree_row,
        rows: project_source_rows(ui),
        tree_rows: active_tree_rows,
        tree_actions: active_tree_actions,
        recovery: active_recovery,
    }
}
