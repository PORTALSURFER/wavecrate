use radiant::prelude as ui;

use super::{FolderBrowserState, VisibleFolder};

impl FolderBrowserState {
    #[cfg(test)]
    pub(in crate::native_app) fn tree_view_start(&self) -> usize {
        self.tree.view_controller.viewport_start()
    }

    pub(super) fn reset_tree_view(&mut self) {
        self.tree.view_controller = ui::VirtualListController::default();
        self.tree.follow_selection.clear();
        self.tree.runtime_viewport_rows = None;
    }

    #[cfg(test)]
    pub(in crate::native_app) fn set_tree_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.visible_folders().len();
        self.tree
            .view_controller
            .set_scroll_offset_for_items(total_items, offset_y, row_height);
    }

    pub(in crate::native_app) fn apply_tree_view_window_change(
        &mut self,
        change: ui::VirtualListWindowChange,
    ) {
        let viewport_rows = change.window.viewport_len().max(1);
        self.tree.runtime_viewport_rows = Some(viewport_rows);
        self.tree.view_controller.apply_window_change(change);
    }

    pub(in crate::native_app) fn tree_view_window(
        &self,
        visible_folders: &[VisibleFolder],
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let viewport_rows = self.tree.runtime_viewport_rows.unwrap_or(viewport_rows);
        ui::resolve_virtual_list_window(ui::VirtualListWindowRequest {
            total_items: visible_folders.len(),
            viewport_len: viewport_rows,
            requested_start: self.tree.view_controller.viewport_start(),
            overscan: overscan_rows,
            focused_index: None,
            previous_start: None,
            guard_band: guard_rows.saturating_add(1),
        })
    }

    pub(in crate::native_app) fn sync_tree_view_to_selection(
        &mut self,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let visible_folders = self.visible_folders();
        self.follow_selected_tree_view(&visible_folders, viewport_rows, overscan_rows, guard_rows)
    }

    fn follow_selected_tree_view(
        &mut self,
        visible_folders: &[VisibleFolder],
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let runtime_viewport_rows = self.tree.runtime_viewport_rows;
        let viewport_rows = runtime_viewport_rows.unwrap_or(viewport_rows);
        let selected_id = self
            .selection
            .selected_collection
            .is_none()
            .then(|| self.selection.selected_folder.clone());
        let selected_index = selected_id.as_ref().and_then(|selected_id| {
            visible_folders
                .iter()
                .position(|folder| folder.id == *selected_id)
        });

        if self.tree.follow_selection.focus_key() == selected_id.as_ref() {
            let projection = ui::VirtualListProjection::for_slice(
                visible_folders,
                viewport_rows,
                overscan_rows,
                guard_rows,
            )
            .with_context_row();
            self.tree.view_controller.configure_projection(projection);
            self.tree.view_controller.clear_focus();
            return self.tree.view_controller.resolve();
        }

        if runtime_viewport_rows.is_some()
            && selected_index.is_some_and(|index| {
                self.tree.view_controller.viewport_contains_index(
                    visible_folders.len(),
                    viewport_rows,
                    index,
                )
            })
        {
            let projection = ui::VirtualListProjection::for_slice(
                visible_folders,
                viewport_rows,
                overscan_rows,
                guard_rows,
            )
            .with_context_row();
            self.tree.follow_selection.remember_focus_key(selected_id);
            self.tree.view_controller.configure_projection(projection);
            self.tree.view_controller.clear_focus();
            return self.tree.view_controller.resolve();
        }

        let projection = ui::VirtualListProjection::for_slice(
            visible_folders,
            viewport_rows,
            overscan_rows,
            guard_rows,
        )
        .with_context_row();
        let focus = ui::VirtualListFocusTarget::new(selected_id, selected_index);
        self.tree
            .view_controller
            .configure_projection_and_focus_changed_optional(
                &mut self.tree.follow_selection,
                projection,
                focus,
            )
    }
}
