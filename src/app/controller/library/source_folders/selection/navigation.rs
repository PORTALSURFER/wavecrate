use super::super::*;

impl AppController {
    pub(crate) fn drop_folder_focus(&mut self) {
        self.ui.sources.folders.focused = None;
        self.ui.sources.folders.scroll_to = None;
        let Some(model) = self.current_folder_model_mut() else {
            return;
        };
        if model.focused.take().is_none() {
            return;
        }
        let snapshot = model.clone();
        let _ = self.patch_current_folder_ui_locally(self.active_folder_pane(), &snapshot, true);
    }

    pub(crate) fn expand_focused_folder(&mut self) {
        let Some(row) = self.ui.sources.folders.focused else {
            return;
        };
        let Some(view) = self.ui.sources.folders.rows.get(row) else {
            return;
        };
        if view.is_root {
            return;
        }
        if view.has_children && !view.expanded {
            self.toggle_folder_expanded(row);
        }
    }

    pub(crate) fn collapse_focused_folder(&mut self) {
        let Some(row) = self.ui.sources.folders.focused else {
            return;
        };
        let Some(view) = self.ui.sources.folders.rows.get(row) else {
            return;
        };
        if view.is_root {
            return;
        }
        if view.has_children && view.expanded {
            self.toggle_folder_expanded(row);
            return;
        }
        if let Some(parent) = view.path.parent()
            && !parent.as_os_str().is_empty()
            && let Some(parent_index) = self
                .ui
                .sources
                .folders
                .rows
                .iter()
                .position(|row| row.path == parent)
        {
            self.focus_folder_row(parent_index);
        }
    }

    pub(crate) fn toggle_folder_expanded(&mut self, row_index: usize) {
        self.clear_drop_target_selection();
        let Some(row) = self.ui.sources.folders.rows.get(row_index).cloned() else {
            return;
        };
        if row.is_root {
            return;
        }
        let path = row.path.clone();
        let snapshot = {
            let Some(model) = self.current_folder_model_mut() else {
                return;
            };
            if !model.available.contains(&path) {
                return;
            }
            if !model.expanded.remove(&path) {
                model.expanded.insert(path.clone());
            }
            model.focused = Some(path.clone());
            model.clone()
        };
        self.ui.sources.folders.focused = Some(row_index);
        self.ui.sources.folders.scroll_to = Some(row_index);
        self.ui.sources.folders.last_focused_path = Some(path.clone());
        self.focus_folder_context();
        let _ = self.patch_current_folder_ui_locally(self.active_folder_pane(), &snapshot, false);
        if let Some(source_id) = self.selected_source_id() {
            self.queue_folder_projection_for_pane(self.active_folder_pane(), source_id, snapshot);
        }
    }

    pub(crate) fn focus_folder_row(&mut self, row_index: usize) {
        self.record_meaningful_ui_transaction("Focus folder row", |controller| {
            controller.clear_drop_target_selection();
            let Some(row) = controller.ui.sources.folders.rows.get(row_index).cloned() else {
                return;
            };
            let path = row.path.clone();
            let snapshot = {
                let Some(model) = controller.current_folder_model_mut() else {
                    return;
                };
                if !row.is_root && !model.available.contains(&path) {
                    return;
                }
                model.focused = Some(path.clone());
                model.clone()
            };
            controller.ui.sources.folders.focused = Some(row_index);
            controller.ui.sources.folders.scroll_to = Some(row_index);
            controller.ui.sources.folders.last_focused_path = Some(path.clone());
            controller.focus_folder_context();
            let _ = controller.patch_current_folder_ui_locally(
                controller.active_folder_pane(),
                &snapshot,
                true,
            );
        });
    }

    pub(crate) fn nudge_folder_selection(&mut self, offset: isize, extend: bool) {
        let Some(current) = self.ui.sources.folders.focused else {
            if !self.ui.sources.folders.rows.is_empty() {
                self.focus_folder_row(0);
            }
            return;
        };
        let len = self.ui.sources.folders.rows.len() as isize;
        if len == 0 {
            return;
        }
        let target = (current as isize + offset).clamp(0, len - 1) as usize;
        if extend {
            // Include the currently focused row plus the target step.
            self.add_folder_to_selection(current);
            self.add_folder_to_selection(target);
        } else {
            self.focus_folder_row(target);
        }
    }

    /// Move folder focus by one UI step without extending selection.
    pub fn nudge_folder_focus_action(&mut self, delta: i8) {
        self.nudge_folder_selection(delta as isize, false);
    }
}
