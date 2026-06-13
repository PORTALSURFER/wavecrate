use super::FolderBrowserUiState;
use super::inline_edit::{
    inline_folder_create_draft_row, inline_folder_draft_location, inline_folder_rename_draft_row,
};
use crate::app_core::actions::{
    NativeFolderRowModel as FolderRowModel, NativeRetainedVec as RetainedVec,
    native_folder_row_model as folder_row_model,
};
use crate::app_core::state::InlineFolderEditKind;

pub(super) fn project_tree_rows(folder_ui: &FolderBrowserUiState) -> RetainedVec<FolderRowModel> {
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
                        inline_folder_create_draft_row(folder_ui, parent, draft_depth, edit),
                    );
                }
            }
            InlineFolderEditKind::Rename { target } => {
                if let Some(target_index) =
                    folder_ui.rows.iter().position(|row| row.path == *target)
                    && let Some(row) = folder_ui.rows.get(target_index)
                    && let Some(projected_row) = projected.get_mut(target_index)
                {
                    *projected_row = inline_folder_rename_draft_row(
                        folder_ui,
                        target,
                        row.depth,
                        target_index,
                        edit,
                    );
                }
            }
        }
    }
    projected.into()
}

pub(super) fn projected_focused_tree_row(
    folder_ui: &FolderBrowserUiState,
    projected_rows: &[FolderRowModel],
) -> Option<usize> {
    let focused = folder_ui.focused?;
    projected_rows
        .iter()
        .position(|row| row.backing_index == Some(focused))
}
