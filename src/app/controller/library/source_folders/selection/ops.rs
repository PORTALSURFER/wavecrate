use super::super::*;
use super::ops_logic::FolderSelectMode;
use super::planning::{
    FolderRangePlan, FolderSelectionChange, plan_add_folder_to_selection,
    plan_clear_folder_selection, plan_range_selection, plan_row_selection,
    plan_toggle_folder_negation,
};

impl AppController {
    pub(crate) fn replace_folder_selection(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Select folder", |controller| {
            controller.clear_drop_target_selection();
            controller.apply_planned_folder_selection(row_index, FolderSelectMode::Replace);
        });
    }

    pub(crate) fn select_folder_range(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Select folder range", |controller| {
            controller.clear_drop_target_selection();
            let rows = controller.ui.sources.folders.rows.clone();
            let anchor_path = controller.current_folder_anchor_path().or_else(|| {
                controller
                    .ui
                    .sources
                    .folders
                    .focused
                    .and_then(|idx| rows.get(idx).map(|row| row.path.clone()))
            });
            let plan = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                plan_range_selection(model, &rows, row_index, anchor_path)
            };
            match plan {
                FolderRangePlan::Change(change) => controller.apply_folder_selection_change(change),
                FolderRangePlan::ReplaceSingle => {
                    controller.apply_planned_folder_selection(row_index, FolderSelectMode::Replace);
                }
                FolderRangePlan::Noop => {}
            }
        });
    }

    pub(crate) fn toggle_folder_row_selection(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Toggle folder selection", |controller| {
            controller.clear_drop_target_selection();
            controller.apply_planned_folder_selection(row_index, FolderSelectMode::Toggle);
        });
    }

    pub(crate) fn toggle_focused_folder_selection(&mut self) {
        let Some(row) = self.ui.sources.folders.focused else {
            return;
        };
        self.toggle_folder_row_selection(row);
    }

    pub(crate) fn clear_folder_selection(&mut self) {
        self.record_meaningful_ui_transaction("Clear folder selection", |controller| {
            controller.clear_drop_target_selection();
            let focused_row = controller.ui.sources.folders.focused;
            let focused_path = focused_row.and_then(|idx| {
                controller
                    .ui
                    .sources
                    .folders
                    .rows
                    .get(idx)
                    .map(|row| row.path.clone())
            });
            let change = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                plan_clear_folder_selection(model, focused_path, focused_row)
            };
            if let Some(change) = change {
                controller.apply_folder_selection_change(change);
            }
        });
    }

    pub(crate) fn add_folder_to_selection(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Add folder selection", |controller| {
            controller.clear_drop_target_selection();
            let rows = controller.ui.sources.folders.rows.clone();
            let change = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                plan_add_folder_to_selection(model, &rows, row_index)
            };
            if let Some(change) = change {
                controller.apply_folder_selection_change(change);
            }
        });
    }

    pub(crate) fn toggle_folder_row_negation(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Toggle folder exclusion", |controller| {
            controller.clear_drop_target_selection();
            let rows = controller.ui.sources.folders.rows.clone();
            let change = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                plan_toggle_folder_negation(model, &rows, row_index)
            };
            if let Some(change) = change {
                controller.apply_folder_selection_change(change);
            }
        });
    }

    pub(crate) fn selected_folder_paths(&self) -> Vec<PathBuf> {
        self.current_folder_model()
            .map(|model| model.selected.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub(in super::super) fn focused_folder_path(&self) -> Option<PathBuf> {
        let row = self.ui.sources.folders.focused?;
        self.ui
            .sources
            .folders
            .rows
            .get(row)
            .map(|row| row.path.clone())
    }

    fn apply_planned_folder_selection(&mut self, row_index: usize, mode: FolderSelectMode) {
        let rows = self.ui.sources.folders.rows.clone();
        let change = {
            let Some(model) = self.current_folder_model_mut() else {
                return;
            };
            plan_row_selection(model, &rows, row_index, mode)
        };
        if let Some(change) = change {
            self.apply_folder_selection_change(change);
        }
    }

    fn apply_folder_selection_change(&mut self, change: FolderSelectionChange) {
        if let Some(index) = change.focused_row {
            self.ui.sources.folders.focused = Some(index);
        }
        self.ui.sources.folders.scroll_to = change.scroll_to_row;
        self.focus_folder_context();
        let _ =
            self.patch_current_folder_ui_locally(self.active_folder_pane(), &change.snapshot, true);
        if change.browser_filters_changed {
            self.rebuild_browser_lists();
        }
    }

    fn current_folder_anchor_path(&self) -> Option<PathBuf> {
        self.current_folder_model()
            .and_then(|model| model.selection_anchor.clone())
    }

    pub(crate) fn focus_folder_by_path(&mut self, path: &Path) {
        self.clear_drop_target_selection();
        let Some(model) = self.current_folder_model_mut() else {
            return;
        };
        if !model.available.contains(path) {
            return;
        }
        model.focused = Some(path.to_path_buf());
        model.selection_anchor = Some(path.to_path_buf());
        model.selected.clear();
        model.selected.insert(path.to_path_buf());
        let snapshot = model.clone();
        if !self.patch_current_folder_ui_locally(self.active_folder_pane(), &snapshot, true)
            && let Some(source_id) = self.selected_source_id()
        {
            self.queue_folder_projection_for_pane(self.active_folder_pane(), source_id, snapshot);
        }
        self.rebuild_browser_lists();
    }
}
