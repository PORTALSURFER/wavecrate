use radiant::prelude as ui;
use std::collections::HashMap;

use super::FolderBrowserState;

impl FolderBrowserState {
    #[cfg(test)]
    pub(in crate::gui_app) fn file_view_start(&self) -> usize {
        self.file_view_controller.viewport_start()
    }

    pub(super) fn reset_file_view(&mut self) {
        self.file_view_controller = ui::VirtualListController::default();
        self.file_view_follow_selection.clear();
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn set_file_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.selected_audio_files().len();
        self.file_view_controller
            .set_scroll_offset_for_items(total_items, offset_y, row_height);
    }

    pub(in crate::gui_app) fn track_file_view_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        self.file_view_controller
            .set_scroll_offset(offset_y, row_height);
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn follow_selected_file_view(
        &mut self,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let audio_files = self.selected_audio_files();
        let focus = ui::VirtualListSliceFocus::from_slice_by(
            &audio_files,
            viewport_rows,
            overscan_rows,
            guard_rows,
            self.selected_file.clone(),
            |file, key| file.id.as_str() == key.as_str(),
        )
        .with_context_row();
        self.file_view_controller
            .configure_slice_focus_changed_optional(&mut self.file_view_follow_selection, focus)
    }

    pub(in crate::gui_app) fn follow_selected_file_view_matching_tags(
        &mut self,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> ui::VirtualListWindow {
        let selected_file = self.selected_file.clone();
        let total_items = self.selected_audio_file_count_matching_tags(tags_by_file);
        if self.file_view_follow_selection.focus_key() == selected_file.as_ref() {
            let projection = ui::VirtualListProjection::new(
                total_items,
                viewport_rows,
                overscan_rows,
                guard_rows,
            )
            .with_context_row();
            self.file_view_controller.configure_projection(projection);
            self.file_view_controller.clear_focus();
            return self.file_view_controller.resolve();
        }

        let projection =
            ui::VirtualListProjection::new(total_items, viewport_rows, overscan_rows, guard_rows)
                .with_context_row();
        let focus = ui::VirtualListFocusTarget::new(
            selected_file,
            self.selected_audio_file_index_matching_tags(tags_by_file),
        );
        self.file_view_controller
            .configure_projection_and_focus_changed_optional(
                &mut self.file_view_follow_selection,
                projection,
                focus,
            )
    }
}
