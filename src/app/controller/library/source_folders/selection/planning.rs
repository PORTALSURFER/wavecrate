use super::super::*;
use super::ops_logic::{
    FolderSelectMode, apply_path_selection, apply_root_selection, insert_folder,
};
use crate::app::state::FolderRowView;

#[derive(Clone, Debug)]
pub(super) struct FolderSelectionChange {
    pub(super) snapshot: FolderBrowserModel,
    pub(super) focused_row: Option<usize>,
    pub(super) scroll_to_row: Option<usize>,
    pub(super) browser_filters_changed: bool,
}

pub(super) fn plan_row_selection(
    model: &mut FolderBrowserModel,
    rows: &[FolderRowView],
    row_index: usize,
    mode: FolderSelectMode,
) -> Option<FolderSelectionChange> {
    let row = rows.get(row_index)?;
    let changed = if row.is_root {
        apply_root_selection(model, mode)
    } else {
        apply_path_selection(model, &row.path, row.has_children, mode)
    };
    Some(change_from_model(model, Some(row_index), changed))
}

pub(super) enum FolderRangePlan {
    Change(FolderSelectionChange),
    ReplaceSingle,
    Noop,
}

pub(super) fn plan_range_selection(
    model: &mut FolderBrowserModel,
    rows: &[FolderRowView],
    row_index: usize,
    anchor_path: Option<PathBuf>,
) -> FolderRangePlan {
    if rows.is_empty() || rows.get(row_index).is_some_and(|row| row.is_root) {
        return FolderRangePlan::ReplaceSingle;
    }
    let Some(anchor_idx) =
        anchor_path.and_then(|path| rows.iter().position(|row| row.path == path))
    else {
        return FolderRangePlan::ReplaceSingle;
    };
    let anchor_idx = anchor_idx.min(rows.len().saturating_sub(1));
    let row_index = row_index.min(rows.len().saturating_sub(1));
    if rows.get(anchor_idx).is_some_and(|row| row.is_root) {
        return FolderRangePlan::ReplaceSingle;
    }
    let start = anchor_idx.min(row_index);
    let end = anchor_idx.max(row_index);
    let selection: Vec<(PathBuf, bool)> = rows[start..=end]
        .iter()
        .filter(|row| !row.is_root)
        .map(|row| (row.path.clone(), row.has_children))
        .collect();
    if selection.is_empty() {
        return FolderRangePlan::Noop;
    }
    model.selected.clear();
    for (path, has_children) in &selection {
        insert_folder(&mut model.selected, path, *has_children);
    }
    model.selection_anchor = Some(rows[anchor_idx].path.clone());
    model.focused = Some(rows[row_index].path.clone());
    FolderRangePlan::Change(change_from_model(model, Some(row_index), true))
}

pub(super) fn plan_add_folder_to_selection(
    model: &mut FolderBrowserModel,
    rows: &[FolderRowView],
    row_index: usize,
) -> Option<FolderSelectionChange> {
    let row = rows.get(row_index)?;
    let before = model.selected.clone();
    if row.is_root {
        let root = PathBuf::new();
        model.selected.insert(root.clone());
        if model.selection_anchor.is_none() {
            model.selection_anchor = Some(root.clone());
        }
        model.focused = Some(root);
    } else {
        if !model.available.contains(&row.path) {
            return None;
        }
        insert_folder(&mut model.selected, &row.path, row.has_children);
        if model.selection_anchor.is_none() {
            model.selection_anchor = Some(row.path.clone());
        }
        model.focused = Some(row.path.clone());
    }
    Some(change_from_model(
        model,
        Some(row_index),
        before != model.selected,
    ))
}

pub(super) fn plan_toggle_folder_negation(
    model: &mut FolderBrowserModel,
    rows: &[FolderRowView],
    row_index: usize,
) -> Option<FolderSelectionChange> {
    let row = rows.get(row_index)?;
    if !row.is_root && !model.available.contains(&row.path) {
        return None;
    }
    let before = model.negated.clone();
    if model.negated.contains(&row.path) {
        model.negated.remove(&row.path);
    } else {
        model.negated.insert(row.path.clone());
    }
    model.focused = Some(row.path.clone());
    Some(change_from_model(
        model,
        Some(row_index),
        before != model.negated,
    ))
}

pub(super) fn plan_clear_folder_selection(
    model: &mut FolderBrowserModel,
    focused_path: Option<PathBuf>,
    focused_row: Option<usize>,
) -> Option<FolderSelectionChange> {
    if model.selected.is_empty() {
        return None;
    }
    model.selected.clear();
    if let Some(focused) = focused_path {
        model.focused = Some(focused.clone());
        if focused.as_os_str().is_empty() {
            model.selection_anchor = None;
        } else {
            model.selection_anchor = Some(focused);
        }
    }
    Some(FolderSelectionChange {
        snapshot: model.clone(),
        focused_row: None,
        scroll_to_row: focused_row,
        browser_filters_changed: true,
    })
}

fn change_from_model(
    model: &FolderBrowserModel,
    focused_row: Option<usize>,
    browser_filters_changed: bool,
) -> FolderSelectionChange {
    FolderSelectionChange {
        snapshot: model.clone(),
        focused_row,
        scroll_to_row: focused_row,
        browser_filters_changed,
    }
}
