use radiant::prelude as ui;

use super::FolderBrowserState;

impl FolderBrowserState {
    #[cfg(test)]
    pub(super) fn file_view_start(&self) -> usize {
        self.file_view_start
    }

    pub(in crate::gui_app) fn set_file_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.selected_audio_files().len();
        if total_items == 0 {
            self.file_view_start = 0;
            return;
        }
        self.file_view_start =
            ui::virtual_list_view_start_for_scroll_offset(offset_y, row_height, total_items);
    }

    pub(in crate::gui_app) fn follow_selected_file_view(
        &mut self,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
    ) -> ui::VirtualListWindow {
        let total_items = self.selected_audio_files().len();
        if total_items == 0 || viewport_rows == 0 {
            self.file_view_start = 0;
            return ui::VirtualListWindow {
                total_items,
                ..Default::default()
            };
        }
        let window = ui::resolve_virtual_list_window(ui::VirtualListWindowRequest {
            total_items,
            viewport_len: viewport_rows,
            requested_start: self.file_view_start,
            overscan: overscan_rows,
            focused_index: self.selected_audio_file_index(),
            previous_start: Some(self.file_view_start),
            guard_band: guard_rows.saturating_add(1),
        });
        self.file_view_start = window.viewport_start;
        window
    }

    pub(in crate::gui_app) fn selected_audio_file_index(&self) -> Option<usize> {
        let selected = self.selected_file.as_deref()?;
        self.selected_audio_files()
            .iter()
            .position(|file| file.id == selected)
    }
}
