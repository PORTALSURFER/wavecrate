use radiant::prelude as ui;

use super::FolderBrowserState;

impl FolderBrowserState {
    #[cfg(test)]
    pub(super) fn file_view_start(&self) -> usize {
        self.file_view_controller.viewport_start()
    }

    pub(super) fn reset_file_view(&mut self) {
        self.file_view_controller = ui::VirtualListController::default();
    }

    pub(in crate::gui_app) fn set_file_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.selected_audio_files().len();
        self.file_view_controller
            .set_scroll_offset_for_items(total_items, offset_y, row_height);
    }

    pub(in crate::gui_app) fn follow_selected_file_view(
        &mut self,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let total_items = self.selected_audio_files().len();
        self.file_view_controller
            .configure_and_focus_optional_with_context_row(
                total_items,
                viewport_rows,
                overscan_rows,
                guard_rows,
                self.selected_audio_file_index(),
            )
    }

    pub(in crate::gui_app) fn selected_audio_file_index(&self) -> Option<usize> {
        let selected = self.selected_file.as_deref()?;
        self.selected_audio_files()
            .iter()
            .position(|file| file.id == selected)
    }
}
