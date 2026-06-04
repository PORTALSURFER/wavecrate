use radiant::prelude as ui;

use super::FolderBrowserState;

impl FolderBrowserState {
    #[cfg(test)]
    pub(super) fn tree_view_start(&self) -> usize {
        self.tree_view_controller.viewport_start()
    }

    pub(super) fn reset_tree_view(&mut self) {
        self.tree_view_controller = ui::VirtualListController::default();
        self.tree_view_follow_selection.clear();
    }

    pub(in crate::gui_app) fn set_tree_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.visible_folders().len();
        self.tree_view_controller
            .set_scroll_offset_for_items(total_items, offset_y, row_height);
    }

    pub(in crate::gui_app) fn follow_selected_tree_view(
        &mut self,
        total_items: usize,
        selected_index: Option<usize>,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let selected_id = selected_index
            .filter(|_| self.selected_collection.is_none())
            .map(|_| self.selected_folder.clone());
        let projection =
            ui::VirtualListProjection::new(total_items, viewport_rows, overscan_rows, guard_rows)
                .with_context_row();
        self.tree_view_controller
            .configure_projection_and_focus_changed_optional(
                &mut self.tree_view_follow_selection,
                projection,
                ui::VirtualListFocusTarget::new(selected_id, selected_index),
            )
    }
}
