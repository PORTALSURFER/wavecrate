use super::super::*;
use super::ops_logic::{FolderSelectMode, apply_path_selection, apply_root_selection};
use crate::app::state::FolderRowView;
use std::path::{Path, PathBuf};

struct FolderActivation {
    path: PathBuf,
    snapshot: FolderBrowserModel,
    selection_changed: bool,
}

impl AppController {
    pub(crate) fn activate_folder_row(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Activate folder", |controller| {
            controller.clear_drop_target_selection();
            let Some(row) = controller.ui.sources.folders.rows.get(row_index).cloned() else {
                return;
            };
            let Some(activation) = controller.prepare_folder_row_activation(&row) else {
                return;
            };
            controller.finish_folder_row_activation(row_index, activation);
        });
    }

    fn prepare_folder_row_activation(&mut self, row: &FolderRowView) -> Option<FolderActivation> {
        let path = row.path.clone();
        let model = self.current_folder_model_mut()?;
        if !row.is_root && !model.available.contains(&path) {
            return None;
        }
        let selection_changed = apply_activation_selection(model, row, &path);
        model.focused = Some(path.clone());
        if should_toggle_folder_on_activation(model, row) {
            toggle_expanded_path(&mut model.expanded, &path);
        }
        Some(FolderActivation {
            path,
            snapshot: model.clone(),
            selection_changed,
        })
    }

    fn finish_folder_row_activation(&mut self, row_index: usize, activation: FolderActivation) {
        self.ui.sources.folders.focused = Some(row_index);
        self.ui.sources.folders.scroll_to = Some(row_index);
        self.ui.sources.folders.last_focused_path = Some(activation.path);
        self.focus_folder_context();
        self.build_folder_rows(&activation.snapshot);
        if activation.selection_changed {
            self.rebuild_browser_lists();
        }
    }
}

fn apply_activation_selection(
    model: &mut FolderBrowserModel,
    row: &FolderRowView,
    path: &Path,
) -> bool {
    if row.is_root {
        apply_root_selection(model, FolderSelectMode::Replace)
    } else {
        apply_path_selection(model, path, row.has_children, FolderSelectMode::Replace)
    }
}

fn should_toggle_folder_on_activation(model: &FolderBrowserModel, row: &FolderRowView) -> bool {
    row.has_children && !row.is_root && model.search_query.trim().is_empty()
}

fn toggle_expanded_path(expanded: &mut std::collections::BTreeSet<PathBuf>, path: &Path) {
    if !expanded.remove(path) {
        expanded.insert(path.to_path_buf());
    }
}
