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
        self.file_view_start = ((offset_y.max(0.0) / row_height.max(1.0)).floor() as usize)
            .min(total_items.saturating_sub(1));
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
        let viewport_rows = viewport_rows.min(total_items).max(1);
        let guard_rows = guard_rows.min(viewport_rows.saturating_sub(1) / 2);
        let overscan_rows = overscan_rows.min(total_items.saturating_sub(viewport_rows));
        let mut viewport_start = self.file_view_start.min(total_items.saturating_sub(1));
        if let Some(focused_index) = self.selected_audio_file_index() {
            let lower_guard = viewport_start.saturating_add(guard_rows);
            let upper_guard = viewport_start
                .saturating_add(viewport_rows.saturating_sub(1))
                .saturating_sub(guard_rows.saturating_add(1));
            if focused_index <= lower_guard {
                viewport_start = focused_index.saturating_sub(guard_rows);
            } else if focused_index >= upper_guard {
                viewport_start = focused_index.saturating_sub(
                    viewport_rows
                        .saturating_sub(1)
                        .saturating_sub(guard_rows.saturating_add(1)),
                );
            }
        }
        self.file_view_start = viewport_start.min(total_items.saturating_sub(1));
        let viewport_end = self
            .file_view_start
            .saturating_add(viewport_rows)
            .min(total_items);
        let window_start = self.file_view_start.saturating_sub(overscan_rows);
        let window_end = viewport_end.saturating_add(overscan_rows).min(total_items);
        ui::VirtualListWindow {
            total_items,
            viewport_start: self.file_view_start,
            viewport_end,
            window_start,
            window_end,
        }
    }

    pub(in crate::gui_app) fn selected_audio_file_index(&self) -> Option<usize> {
        let selected = self.selected_file.as_deref()?;
        self.selected_audio_files()
            .iter()
            .position(|file| file.id == selected)
    }
}
