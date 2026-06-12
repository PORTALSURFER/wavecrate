use super::tree_rows::{project_tree_rows, projected_focused_tree_row};
use super::*;

pub(super) fn project_folder_pane(
    controller: &AppController,
    pane: FolderPaneId,
) -> FolderPaneModel {
    let ui = &controller.ui;
    let browser = folder_browser_ui_for_projection(ui, pane);
    let projected_tree_rows = project_tree_rows(browser);
    let focused_tree_row = projected_focused_tree_row(browser, &projected_tree_rows);
    let source = ui
        .sources
        .folder_pane(pane)
        .source_id
        .as_ref()
        .and_then(|source_id| ui.sources.rows.iter().find(|row| row.id == *source_id))
        .or_else(|| {
            (ui.sources.active_folder_pane == pane)
                .then(|| {
                    ui.sources
                        .selected
                        .and_then(|index| ui.sources.rows.get(index))
                })
                .flatten()
        });
    let has_item = source.is_some();
    let can_manage_folder = browser
        .focused
        .and_then(|index| browser.rows.get(index))
        .is_some_and(|row| !row.is_root);

    FolderPaneModel {
        pane: project_folder_pane_id(pane),
        title: match pane {
            FolderPaneId::Upper => String::from("Upper"),
            FolderPaneId::Lower => String::from("Lower"),
        },
        item_label: source
            .map(|row| row.name.clone())
            .unwrap_or_else(|| String::from("No source")),
        item_detail: source.map(|row| row.path.clone()).unwrap_or_default(),
        active: ui.sources.active_folder_pane == pane,
        has_item,
        loading: ui.sources.folder_pane(pane).loading,
        projecting: ui.sources.folder_pane(pane).projecting,
        mutation_busy: ui
            .sources
            .folder_pane(pane)
            .source_id
            .as_ref()
            .is_some_and(|source_id| controller.source_has_pending_file_mutations(source_id)),
        tree_search_query: browser.search_query.clone(),
        show_all_items: browser.show_all_folders,
        can_toggle_show_all_items: has_item,
        flattened_view: browser.flattened_view,
        can_toggle_flattened_view: has_item,
        focused_tree_row,
        tree_rows: projected_tree_rows,
        tree_actions: FolderActionsModel {
            can_create_child: has_item,
            can_create_root: has_item || ui.sources.rows.is_empty(),
            can_rename: can_manage_folder,
            can_delete: can_manage_folder,
            can_restore_retained: !browser.delete_recovery.retained_entries.is_empty()
                && !browser.delete_recovery.in_progress,
            can_purge_retained: !browser.delete_recovery.retained_entries.is_empty()
                && !browser.delete_recovery.in_progress,
            can_clear_history: !browser.delete_recovery.entries.is_empty()
                && !browser.delete_recovery.in_progress,
        },
        recovery: FolderRecoveryModel {
            in_progress: browser.delete_recovery.in_progress,
            entry_count: browser.delete_recovery.entries.len(),
            retained_count: browser.delete_recovery.retained_entries.len(),
        },
    }
}

fn folder_browser_ui_for_projection(ui: &UiState, pane: FolderPaneId) -> &FolderBrowserUiState {
    if ui.sources.active_folder_pane == pane {
        &ui.sources.folders
    } else {
        &ui.sources.folder_pane(pane).browser
    }
}

pub(super) fn project_folder_pane_id(pane: FolderPaneId) -> FolderPaneIdModel {
    match pane {
        FolderPaneId::Upper => FolderPaneIdModel::Upper,
        FolderPaneId::Lower => FolderPaneIdModel::Lower,
    }
}
