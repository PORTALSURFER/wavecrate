//! Source/folder sidebar projection helpers.

use super::*;
use crate::app_core::app_api::state::{FolderBrowserUiState, FolderPaneId};
use std::path::{Path, PathBuf};

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
        loading_row: ui
            .sources
            .loading_source_id
            .as_ref()
            .and_then(|source_id| ui.sources.rows.iter().position(|row| row.id == *source_id)),
        mutation_busy_row: ui
            .sources
            .rows
            .iter()
            .position(|row| controller.source_has_pending_file_mutations(&row.id)),
        focused_tree_row: active_focused_tree_row,
        rows: ui
            .sources
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                let upper_assigned = ui
                    .sources
                    .folder_pane(FolderPaneId::Upper)
                    .source_id
                    .as_ref()
                    .is_some_and(|source_id| *source_id == row.id);
                let lower_assigned = ui
                    .sources
                    .folder_pane(FolderPaneId::Lower)
                    .source_id
                    .as_ref()
                    .is_some_and(|source_id| *source_id == row.id);
                SourceRowModel::new(
                    row.name.clone(),
                    row.path.clone(),
                    ui.sources
                        .selected
                        .is_some_and(|selected| selected == row_index),
                    row.missing,
                )
                .with_pane_assignment(upper_assigned, lower_assigned)
            })
            .collect::<Vec<_>>()
            .into(),
        tree_rows: active_tree_rows,
        tree_actions: active_tree_actions,
        recovery: active_recovery,
    }
}

fn project_folder_pane(controller: &AppController, pane: FolderPaneId) -> FolderPaneModel {
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

fn project_folder_pane_id(pane: FolderPaneId) -> FolderPaneIdModel {
    match pane {
        FolderPaneId::Upper => FolderPaneIdModel::Upper,
        FolderPaneId::Lower => FolderPaneIdModel::Lower,
    }
}

fn project_tree_rows(folder_ui: &FolderBrowserUiState) -> RetainedVec<FolderRowModel> {
    let mut projected: Vec<FolderRowModel> = folder_ui
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            folder_row_model(
                row.name.clone(),
                row.path.display().to_string(),
                row.depth,
                row.selected,
                folder_ui
                    .focused
                    .is_some_and(|focused| focused == row_index),
                row.is_root,
                row.has_children,
                row.expanded,
            )
            .with_backing_index(row_index)
        })
        .collect();
    if let Some(edit) = folder_ui.inline_edit.as_ref() {
        match &edit.kind {
            InlineFolderEditKind::Create { parent } => {
                if let Some((insert_index, draft_depth)) =
                    inline_folder_draft_location(folder_ui, parent)
                {
                    projected.insert(
                        insert_index,
                        inline_folder_draft_row(
                            FolderRowKind::CreateDraft,
                            draft_depth,
                            edit.name.clone(),
                            String::from("New folder name"),
                            folder_create_validation_error(folder_ui, parent, &edit.name),
                            edit.focus_requested,
                            edit.select_all_on_focus_requested,
                            None,
                        ),
                    );
                }
            }
            InlineFolderEditKind::Rename { target } => {
                if let Some(target_index) =
                    folder_ui.rows.iter().position(|row| row.path == *target)
                    && let Some(row) = folder_ui.rows.get(target_index)
                    && let Some(projected_row) = projected.get_mut(target_index)
                {
                    *projected_row = inline_folder_draft_row(
                        FolderRowKind::RenameDraft,
                        row.depth,
                        edit.name.clone(),
                        String::from("Folder name"),
                        folder_rename_validation_error(folder_ui, target, &edit.name),
                        edit.focus_requested,
                        edit.select_all_on_focus_requested,
                        Some(target_index),
                    );
                }
            }
        }
    }
    projected.into()
}

fn projected_focused_tree_row(
    folder_ui: &FolderBrowserUiState,
    projected_rows: &[FolderRowModel],
) -> Option<usize> {
    let focused = folder_ui.focused?;
    projected_rows
        .iter()
        .position(|row| row.backing_index == Some(focused))
}

fn inline_folder_draft_location(
    folder_ui: &FolderBrowserUiState,
    parent: &Path,
) -> Option<(usize, usize)> {
    if parent.as_os_str().is_empty() {
        let root_index = folder_ui.rows.iter().position(|row| row.is_root)?;
        return Some((root_index + 1, 1));
    }
    let parent_index = folder_ui.rows.iter().position(|row| row.path == parent)?;
    let parent_depth = folder_ui.rows.get(parent_index)?.depth;
    Some((parent_index + 1, parent_depth + 1))
}

fn inline_folder_draft_row(
    kind: FolderRowKind,
    depth: usize,
    input_value: String,
    input_placeholder: String,
    input_error: Option<String>,
    input_focused: bool,
    select_all_on_focus: bool,
    backing_index: Option<usize>,
) -> FolderRowModel {
    let mut row = match kind {
        FolderRowKind::CreateDraft => FolderRowModel::create_draft(
            depth,
            input_value,
            input_placeholder,
            input_error,
            input_focused,
        ),
        FolderRowKind::RenameDraft => FolderRowModel::rename_draft(
            depth,
            input_value,
            input_placeholder,
            input_error,
            input_focused,
        ),
        FolderRowKind::Existing => FolderRowModel::from_parts(Default::default()),
    };
    row.backing_index = backing_index;
    row.input.select_all_on_focus = select_all_on_focus;
    row
}

fn normalize_folder_name_input(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(String::from("Folder name cannot be empty"));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(String::from("Folder name is invalid"));
    }
    if trimmed.contains(['/', '\\']) {
        return Err(String::from("Folder name cannot contain path separators"));
    }
    Ok(trimmed.to_string())
}

fn display_relative_folder_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn folder_exists_in_rows(folder_ui: &FolderBrowserUiState, relative_path: &Path) -> bool {
    folder_ui.rows.iter().any(|row| row.path == relative_path)
}

fn folder_create_validation_error(
    folder_ui: &FolderBrowserUiState,
    parent: &Path,
    name: &str,
) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let relative = if parent.as_os_str().is_empty() {
        PathBuf::from(&normalized)
    } else {
        parent.join(&normalized)
    };
    folder_exists_in_rows(folder_ui, &relative).then_some(format!(
        "Folder already exists: {}",
        display_relative_folder_path(&relative)
    ))
}

fn folder_rename_validation_error(
    folder_ui: &FolderBrowserUiState,
    target: &Path,
    name: &str,
) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let renamed = folder_with_name(target, &normalized);
    if renamed == target {
        return None;
    }
    folder_exists_in_rows(folder_ui, &renamed).then_some(format!(
        "Folder already exists: {}",
        display_relative_folder_path(&renamed)
    ))
}

fn folder_with_name(target: &Path, name: &str) -> PathBuf {
    target.parent().map_or_else(
        || PathBuf::from(name),
        |parent| {
            if parent.as_os_str().is_empty() {
                PathBuf::from(name)
            } else {
                parent.join(name)
            }
        },
    )
}
