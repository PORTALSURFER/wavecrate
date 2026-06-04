use radiant::prelude as ui;

use super::{FileEntry, FolderBrowserState};

impl FolderBrowserState {
    #[cfg(test)]
    pub(in crate::gui_app) fn file_view_start(&self) -> usize {
        self.file_view_controller.viewport_start()
    }

    pub(super) fn reset_file_view(&mut self) {
        self.file_view_controller = ui::VirtualListController::default();
        self.file_view_follow_selection.clear();
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
        let audio_files = self.selected_audio_files();
        let total_items = audio_files.len();
        let selected_index =
            selected_audio_file_index_in(&audio_files, self.selected_file.as_deref());
        let selected_id = selected_index.and_then(|_| self.selected_file.clone());
        let projection =
            ui::VirtualListProjection::new(total_items, viewport_rows, overscan_rows, guard_rows)
                .with_context_row();
        self.file_view_controller
            .configure_projection_and_focus_changed_optional(
                &mut self.file_view_follow_selection,
                projection,
                ui::VirtualListFocusTarget::new(selected_id, selected_index),
            )
    }

    pub(in crate::gui_app) fn selected_audio_file_index(&self) -> Option<usize> {
        selected_audio_file_index_in(&self.selected_audio_files(), self.selected_file.as_deref())
    }
}

fn selected_audio_file_index_in(files: &[&FileEntry], selected: Option<&str>) -> Option<usize> {
    let selected = selected?;
    files.iter().position(|file| file.id == selected)
}
