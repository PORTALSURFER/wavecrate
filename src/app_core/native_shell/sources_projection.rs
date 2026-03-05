//! Source/folder sidebar projection helpers.

use super::*;

/// Project source/folder panel data for the native sidebar.
pub(crate) fn project_sources_model(ui: &UiState) -> SourcesPanelModel {
    let focused_folder = ui
        .sources
        .folders
        .focused
        .and_then(|index| ui.sources.folders.rows.get(index).cloned());
    let source_selected = ui.sources.selected.is_some();
    let has_sources = !ui.sources.rows.is_empty();
    let can_manage_folder = focused_folder.as_ref().is_some_and(|row| !row.is_root);
    SourcesPanelModel {
        header: format!("Sources ({})", ui.sources.rows.len()),
        search_query: ui.sources.folders.search_query.clone(),
        folder_search_query: ui.sources.folders.search_query.clone(),
        selected_row: ui.sources.selected,
        focused_folder_row: ui.sources.folders.focused,
        rows: ui
            .sources
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                SourceRowModel::new(
                    row.name.clone(),
                    row.path.clone(),
                    ui.sources
                        .selected
                        .is_some_and(|selected| selected == row_index),
                    row.missing,
                )
            })
            .collect(),
        folder_rows: ui
            .sources
            .folders
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                FolderRowModel::new(
                    row.name.clone(),
                    row.path.display().to_string(),
                    row.depth,
                    row.selected,
                    ui.sources
                        .folders
                        .focused
                        .is_some_and(|focused| focused == row_index),
                    row.is_root,
                    row.has_children,
                    row.expanded,
                )
            })
            .collect(),
        folder_actions: FolderActionsModel {
            can_create_folder: source_selected,
            can_create_folder_at_root: source_selected || !has_sources,
            can_rename_folder: can_manage_folder,
            can_delete_folder: can_manage_folder,
            can_clear_recovery_log: !ui.sources.folders.delete_recovery.entries.is_empty()
                && !ui.sources.folders.delete_recovery.in_progress,
        },
        folder_recovery: FolderRecoveryModel {
            in_progress: ui.sources.folders.delete_recovery.in_progress,
            entry_count: ui.sources.folders.delete_recovery.entries.len(),
        },
    }
}
