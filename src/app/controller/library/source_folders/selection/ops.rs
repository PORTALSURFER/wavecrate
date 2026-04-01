use super::super::*;
use super::ops_logic::{
    FolderSelectMode, apply_path_selection, apply_root_selection, insert_folder,
};
use crate::app::state::FolderRowView;

impl AppController {
    pub(crate) fn replace_folder_selection(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Select folder", |controller| {
            controller.clear_drop_target_selection();
            controller.apply_folder_selection(row_index, FolderSelectMode::Replace);
        });
    }

    pub(crate) fn select_folder_range(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Select folder range", |controller| {
            controller.clear_drop_target_selection();
            let rows = controller.ui.sources.folders.rows.clone();
            if rows.is_empty() {
                return;
            }
            if rows.get(row_index).is_some_and(|row| row.is_root) {
                controller.apply_folder_selection(row_index, FolderSelectMode::Replace);
                return;
            }
            let Some(anchor_idx) = controller.folder_anchor_index(&rows) else {
                controller.apply_folder_selection(row_index, FolderSelectMode::Replace);
                return;
            };
            let anchor_idx = anchor_idx.min(rows.len().saturating_sub(1));
            let row_index = row_index.min(rows.len().saturating_sub(1));
            if rows.get(anchor_idx).is_some_and(|row| row.is_root) {
                controller.apply_folder_selection(row_index, FolderSelectMode::Replace);
                return;
            }
            let start = anchor_idx.min(row_index);
            let end = anchor_idx.max(row_index);
            let selection: Vec<(PathBuf, bool)> = rows[start..=end]
                .iter()
                .filter(|row| !row.is_root)
                .map(|row| (row.path.clone(), row.has_children))
                .collect();
            if selection.is_empty() {
                return;
            }
            let (snapshot, selection_changed) = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                model.selected.clear();
                for (path, has_children) in &selection {
                    insert_folder(&mut model.selected, path, *has_children);
                }
                model.selection_anchor = Some(rows[anchor_idx].path.clone());
                model.focused = Some(rows[row_index].path.clone());
                (model.clone(), true)
            };
            controller.ui.sources.folders.focused = Some(row_index);
            controller.ui.sources.folders.scroll_to = Some(row_index);
            controller.focus_folder_context();
            controller.build_folder_rows(&snapshot);
            if selection_changed {
                controller.rebuild_browser_lists();
            }
        });
    }

    pub(crate) fn toggle_folder_row_selection(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Toggle folder selection", |controller| {
            controller.clear_drop_target_selection();
            controller.apply_folder_selection(row_index, FolderSelectMode::Toggle);
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
            let focused_path = controller.ui.sources.folders.focused.and_then(|idx| {
                controller
                    .ui
                    .sources
                    .folders
                    .rows
                    .get(idx)
                    .map(|row| row.path.clone())
            });
            let snapshot = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                if model.selected.is_empty() {
                    return;
                }
                model.selected.clear();
                if let Some(focused) = focused_path.clone() {
                    model.focused = Some(focused.clone());
                    if focused.as_os_str().is_empty() {
                        model.selection_anchor = None;
                    } else {
                        model.selection_anchor = Some(focused);
                    }
                }
                model.clone()
            };
            controller.ui.sources.folders.scroll_to = controller.ui.sources.folders.focused;
            controller.build_folder_rows(&snapshot);
            controller.rebuild_browser_lists();
        });
    }

    pub(crate) fn add_folder_to_selection(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Add folder selection", |controller| {
            controller.clear_drop_target_selection();
            let Some(row) = controller.ui.sources.folders.rows.get(row_index).cloned() else {
                return;
            };
            if row.is_root {
                let (snapshot, selection_changed) = {
                    let Some(model) = controller.current_folder_model_mut() else {
                        return;
                    };
                    let before = model.selected.clone();
                    model.selected.insert(PathBuf::new());
                    if model.selection_anchor.is_none() {
                        model.selection_anchor = Some(PathBuf::new());
                    }
                    model.focused = Some(PathBuf::new());
                    (model.clone(), before != model.selected)
                };
                controller.ui.sources.folders.focused = Some(row_index);
                controller.ui.sources.folders.scroll_to = Some(row_index);
                controller.focus_folder_context();
                controller.build_folder_rows(&snapshot);
                if selection_changed {
                    controller.rebuild_browser_lists();
                }
                return;
            }
            let path = row.path.clone();
            let (snapshot, selection_changed) = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                if !model.available.contains(&path) {
                    return;
                }
                let before = model.selected.clone();
                insert_folder(&mut model.selected, &path, row.has_children);
                if model.selection_anchor.is_none() {
                    model.selection_anchor = Some(path.clone());
                }
                model.focused = Some(path.clone());
                let changed = before != model.selected;
                (model.clone(), changed)
            };
            controller.ui.sources.folders.focused = Some(row_index);
            controller.ui.sources.folders.scroll_to = Some(row_index);
            controller.focus_folder_context();
            controller.build_folder_rows(&snapshot);
            if selection_changed {
                controller.rebuild_browser_lists();
            }
        });
    }

    pub(crate) fn toggle_folder_row_negation(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Toggle folder exclusion", |controller| {
            controller.clear_drop_target_selection();
            let Some(row) = controller.ui.sources.folders.rows.get(row_index).cloned() else {
                return;
            };
            let path = row.path.clone();
            let (snapshot, negation_changed) = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                if !row.is_root && !model.available.contains(&path) {
                    return;
                }
                let before = model.negated.clone();
                if model.negated.contains(&path) {
                    model.negated.remove(&path);
                } else {
                    model.negated.insert(path.clone());
                }
                model.focused = Some(path.clone());
                (model.clone(), before != model.negated)
            };
            controller.ui.sources.folders.focused = Some(row_index);
            controller.ui.sources.folders.scroll_to = Some(row_index);
            controller.focus_folder_context();
            controller.build_folder_rows(&snapshot);
            if negation_changed {
                controller.rebuild_browser_lists();
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

    fn apply_folder_selection(&mut self, row_index: usize, mode: FolderSelectMode) {
        let Some(row) = self.ui.sources.folders.rows.get(row_index).cloned() else {
            return;
        };
        if row.is_root {
            let (snapshot, selection_changed) = {
                let Some(model) = self.current_folder_model_mut() else {
                    return;
                };
                let changed = apply_root_selection(model, mode);
                (model.clone(), changed)
            };
            self.ui.sources.folders.focused = Some(row_index);
            self.ui.sources.folders.scroll_to = Some(row_index);
            self.focus_folder_context();
            self.build_folder_rows(&snapshot);
            if selection_changed {
                self.rebuild_browser_lists();
            }
            return;
        }
        let path = row.path.clone();
        let (snapshot, selection_changed) = {
            let Some(model) = self.current_folder_model_mut() else {
                return;
            };
            let changed = apply_path_selection(model, &path, row.has_children, mode);
            (model.clone(), changed)
        };
        self.ui.sources.folders.focused = Some(row_index);
        self.ui.sources.folders.scroll_to = Some(row_index);
        self.focus_folder_context();
        self.build_folder_rows(&snapshot);
        if selection_changed {
            self.rebuild_browser_lists();
        }
    }

    fn folder_anchor_index(&self, rows: &[FolderRowView]) -> Option<usize> {
        let anchor_path = self.current_folder_anchor_path().or_else(|| {
            self.ui
                .sources
                .folders
                .focused
                .and_then(|idx| rows.get(idx).map(|r| r.path.clone()))
        });
        anchor_path.and_then(|path| rows.iter().position(|row| row.path == path))
    }

    fn current_folder_anchor_path(&self) -> Option<PathBuf> {
        self.current_folder_model()
            .and_then(|model| model.selection_anchor.clone())
    }

    pub(in super::super) fn focus_folder_by_path(&mut self, path: &Path) {
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
        self.build_folder_rows(&snapshot);
        self.rebuild_browser_lists();
    }
}
