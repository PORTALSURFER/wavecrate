use radiant::prelude as ui;

use super::{FolderBrowserState, VisibleFolder};

impl FolderBrowserState {
    #[cfg(test)]
    pub(super) fn tree_view_start(&self) -> usize {
        self.tree.view_controller.viewport_start()
    }

    pub(super) fn reset_tree_view(&mut self) {
        self.tree.view_controller = ui::VirtualListController::default();
        self.tree.follow_selection.clear();
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
        self.tree
            .view_controller
            .set_total_items(change.window.total_items);
        self.tree
            .view_controller
            .set_viewport_start(change.window.viewport_start);
    }

    pub(in crate::native_app) fn tree_view_window(
        &self,
        visible_folders: &[VisibleFolder],
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
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
        let selected_id = self
            .selection
            .selected_collection
            .is_none()
            .then(|| self.selection.selected_folder.clone());
        let focus = ui::VirtualListSliceFocus::from_slice_by(
            visible_folders,
            viewport_rows,
            overscan_rows,
            guard_rows,
            selected_id,
            |folder, key| folder.id.as_str() == key.as_str(),
        )
        .with_context_row();
        self.tree
            .view_controller
            .configure_slice_focus_changed_optional(&mut self.tree.follow_selection, focus)
    }
}
