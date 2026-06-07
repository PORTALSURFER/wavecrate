use radiant::prelude as ui;

use super::{FolderBrowserState, VisibleFolder};

impl FolderBrowserState {
    #[cfg(test)]
    pub(super) fn tree_view_start(&self) -> usize {
        self.tree_view_controller.viewport_start()
    }

    pub(super) fn reset_tree_view(&mut self) {
        self.tree_view_controller = ui::VirtualListController::default();
        self.tree_view_follow_selection.clear();
    }

    pub(in crate::native_app) fn set_tree_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.visible_folders().len();
        self.tree_view_controller
            .set_scroll_offset_for_items(total_items, offset_y, row_height);
    }

    pub(in crate::native_app::browser::folder_browser) fn follow_selected_tree_view(
        &mut self,
        visible_folders: &[VisibleFolder],
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let selected_id = self
            .selected_collection
            .is_none()
            .then(|| self.selected_folder.clone());
        let focus = ui::VirtualListSliceFocus::from_slice_by(
            visible_folders,
            viewport_rows,
            overscan_rows,
            guard_rows,
            selected_id,
            |folder, key| folder.id.as_str() == key.as_str(),
        )
        .with_context_row();
        self.tree_view_controller
            .configure_slice_focus_changed_optional(&mut self.tree_view_follow_selection, focus)
    }
}
