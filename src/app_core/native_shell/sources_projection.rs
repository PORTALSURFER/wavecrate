//! Source/folder sidebar projection helpers.

use super::*;
use std::path::{Path, PathBuf};

/// Project source/folder panel data for the native sidebar.
pub(crate) fn project_sources_model(ui: &UiState) -> SourcesPanelModel {
    let source_selected = ui.sources.selected.is_some();
    let has_sources = !ui.sources.rows.is_empty();
    let can_manage_folder = ui
        .sources
        .folders
        .focused
        .and_then(|index| ui.sources.folders.rows.get(index))
        .is_some_and(|row| !row.is_root);
    let projected_folder_rows = project_folder_rows(ui);
    let focused_folder_row = projected_focused_folder_row(ui, &projected_folder_rows);
    SourcesPanelModel {
        header: format!("Sources ({})", ui.sources.rows.len()),
        search_query: ui.sources.folders.search_query.clone(),
        folder_search_query: ui.sources.folders.search_query.clone(),
        show_all_folders: ui.sources.folders.show_all_folders,
        can_toggle_show_all_folders: source_selected,
        selected_row: ui.sources.selected,
        focused_folder_row,
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
        folder_rows: projected_folder_rows,
        folder_actions: FolderActionsModel {
            can_create_folder: source_selected,
            can_create_folder_at_root: source_selected || !has_sources,
            can_rename_folder: can_manage_folder,
            can_delete_folder: can_manage_folder,
            can_restore_retained_deletes: !ui
                .sources
                .folders
                .delete_recovery
                .retained_entries
                .is_empty()
                && !ui.sources.folders.delete_recovery.in_progress,
            can_purge_retained_deletes: !ui
                .sources
                .folders
                .delete_recovery
                .retained_entries
                .is_empty()
                && !ui.sources.folders.delete_recovery.in_progress,
            can_clear_recovery_log: !ui.sources.folders.delete_recovery.entries.is_empty()
                && !ui.sources.folders.delete_recovery.in_progress,
        },
        folder_recovery: FolderRecoveryModel {
            in_progress: ui.sources.folders.delete_recovery.in_progress,
            entry_count: ui.sources.folders.delete_recovery.entries.len(),
            retained_count: ui.sources.folders.delete_recovery.retained_entries.len(),
        },
    }
}

fn project_folder_rows(ui: &UiState) -> Vec<FolderRowModel> {
    let mut projected: Vec<FolderRowModel> = ui
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
            .with_source_index(row_index)
        })
        .collect();
    if let Some(edit) = ui.sources.folders.inline_edit.as_ref() {
        match &edit.kind {
            InlineFolderEditKind::Create { parent } => {
                if let Some((insert_index, draft_depth)) = inline_folder_draft_location(ui, parent) {
                    projected.insert(
                        insert_index,
                        inline_folder_draft_row(
                            FolderRowKind::CreateDraft,
                            draft_depth,
                            edit.name.clone(),
                            String::from("New folder name"),
                            folder_create_validation_error(ui, parent, &edit.name),
                            edit.focus_requested,
                            edit.select_all_on_focus_requested,
                            None,
                        ),
                    );
                }
            }
            InlineFolderEditKind::Rename { target } => {
                if let Some(target_index) = ui
                    .sources
                    .folders
                    .rows
                    .iter()
                    .position(|row| row.path == *target)
                    && let Some(row) = ui.sources.folders.rows.get(target_index)
                    && let Some(projected_row) = projected.get_mut(target_index)
                {
                    *projected_row = inline_folder_draft_row(
                        FolderRowKind::RenameDraft,
                        row.depth,
                        edit.name.clone(),
                        String::from("Folder name"),
                        folder_rename_validation_error(ui, target, &edit.name),
                        edit.focus_requested,
                        edit.select_all_on_focus_requested,
                        Some(target_index),
                    );
                }
            }
        }
    }
    projected
}

fn projected_focused_folder_row(ui: &UiState, projected_rows: &[FolderRowModel]) -> Option<usize> {
    let focused = ui.sources.folders.focused?;
    projected_rows
        .iter()
        .position(|row| row.source_index == Some(focused))
}

fn inline_folder_draft_location(ui: &UiState, parent: &Path) -> Option<(usize, usize)> {
    if parent.as_os_str().is_empty() {
        let root_index = ui.sources.folders.rows.iter().position(|row| row.is_root)?;
        return Some((root_index + 1, 1));
    }
    let parent_index = ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == parent)?;
    let parent_depth = ui.sources.folders.rows.get(parent_index)?.depth;
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
    source_index: Option<usize>,
) -> FolderRowModel {
    FolderRowModel {
        label: String::new(),
        detail: String::new(),
        depth,
        selected: false,
        focused: false,
        is_root: false,
        has_children: false,
        expanded: false,
        kind,
        source_index,
        input_value: Some(input_value),
        input_placeholder: Some(input_placeholder),
        input_error,
        input_focused,
        select_all_on_focus,
    }
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

fn folder_exists_in_rows(ui: &UiState, relative_path: &Path) -> bool {
    ui.sources
        .folders
        .rows
        .iter()
        .any(|row| row.path == relative_path)
}

fn folder_create_validation_error(ui: &UiState, parent: &Path, name: &str) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let relative = if parent.as_os_str().is_empty() {
        PathBuf::from(&normalized)
    } else {
        parent.join(&normalized)
    };
    folder_exists_in_rows(ui, &relative).then_some(format!(
        "Folder already exists: {}",
        display_relative_folder_path(&relative)
    ))
}

fn folder_rename_validation_error(ui: &UiState, target: &Path, name: &str) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let renamed = folder_with_name(target, &normalized);
    if renamed == target {
        return None;
    }
    folder_exists_in_rows(ui, &renamed).then_some(format!(
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
