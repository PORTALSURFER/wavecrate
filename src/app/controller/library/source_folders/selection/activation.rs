use super::super::*;
use super::ops_logic::{FolderSelectMode, apply_path_selection, apply_root_selection};
use crate::app::state::FolderRowView;
use std::path::{Path, PathBuf};

struct FolderActivation {
    path: PathBuf,
    snapshot: FolderBrowserModel,
    selection_changed: bool,
    structure_changed: bool,
}

impl AppController {
    pub(crate) fn activate_folder_row(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Activate folder", |controller| {
            controller.clear_drop_target_selection();
            let Some(row) = controller.ui.sources.folders.rows.get(row_index).cloned() else {
                return;
            };
            let repeat_click = controller.repeat_folder_row_click(row_index);
            let Some(activation) = controller.prepare_folder_row_activation(&row, repeat_click)
            else {
                return;
            };
            controller.finish_folder_row_activation(row_index, activation);
        });
    }

    fn prepare_folder_row_activation(
        &mut self,
        row: &FolderRowView,
        repeat_click: bool,
    ) -> Option<FolderActivation> {
        let path = row.path.clone();
        let model = self.current_folder_model_mut()?;
        if !row.is_root && !model.available.contains(&path) {
            return None;
        }
        let selection_changed = apply_activation_selection(model, row, &path);
        model.focused = Some(path.clone());
        let structure_changed = should_toggle_folder_on_activation(model, row, repeat_click);
        if structure_changed {
            toggle_expanded_path(&mut model.expanded, &path);
        }
        Some(FolderActivation {
            path,
            snapshot: model.clone(),
            selection_changed,
            structure_changed,
        })
    }

    fn repeat_folder_row_click(&self, row_index: usize) -> bool {
        self.ui.sources.folders.focused == Some(row_index)
            && matches!(
                self.ui.focus.context,
                crate::app::state::FocusContext::SourceFolders
            )
    }

    fn finish_folder_row_activation(&mut self, row_index: usize, activation: FolderActivation) {
        self.ui.sources.folders.focused = Some(row_index);
        self.ui.sources.folders.scroll_to = Some(row_index);
        self.ui.sources.folders.last_focused_path = Some(activation.path);
        self.focus_folder_context();
        let _ = self.patch_current_folder_ui_locally(
            self.active_folder_pane(),
            &activation.snapshot,
            false,
        );
        if activation.structure_changed
            && let Some(source_id) = self.selected_source_id()
        {
            self.queue_folder_projection_for_pane(
                self.active_folder_pane(),
                source_id,
                activation.snapshot.clone(),
            );
        }
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

fn should_toggle_folder_on_activation(
    model: &FolderBrowserModel,
    row: &FolderRowView,
    repeat_click: bool,
) -> bool {
    row.has_children
        && !row.is_root
        && model.search_query.trim().is_empty()
        && (!row.expanded || repeat_click)
}

fn toggle_expanded_path(expanded: &mut std::collections::BTreeSet<PathBuf>, path: &Path) {
    if !expanded.remove(path) {
        expanded.insert(path.to_path_buf());
    }
}
