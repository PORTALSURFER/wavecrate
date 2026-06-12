#![allow(clippy::too_many_arguments)]

use super::folder_name_validation::{
    folder_create_validation_error, folder_rename_validation_error,
};
use super::*;
use crate::app_core::app_api::state::InlineFolderEdit;

pub(super) fn inline_folder_draft_location(
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

pub(super) fn inline_folder_create_draft_row(
    folder_ui: &FolderBrowserUiState,
    parent: &Path,
    depth: usize,
    edit: &InlineFolderEdit,
) -> FolderRowModel {
    inline_folder_draft_row(
        FolderRowKind::CreateDraft,
        depth,
        edit.name.clone(),
        String::from("New folder name"),
        folder_create_validation_error(folder_ui, parent, &edit.name),
        edit.focus_requested,
        edit.select_all_on_focus_requested,
        None,
    )
}

pub(super) fn inline_folder_rename_draft_row(
    folder_ui: &FolderBrowserUiState,
    target: &Path,
    depth: usize,
    target_index: usize,
    edit: &InlineFolderEdit,
) -> FolderRowModel {
    inline_folder_draft_row(
        FolderRowKind::RenameDraft,
        depth,
        edit.name.clone(),
        String::from("Folder name"),
        folder_rename_validation_error(folder_ui, target, &edit.name),
        edit.focus_requested,
        edit.select_all_on_focus_requested,
        Some(target_index),
    )
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
