use radiant::prelude as ui;
use std::collections::HashMap;

use super::FolderBrowserState;

impl FolderBrowserState {
    #[cfg(test)]
    pub(in crate::native_app) fn file_view_start(&self) -> usize {
        self.sample_list.view_controller.viewport_start()
    }

    pub(super) fn reset_file_view(&mut self) {
        self.sample_list.reset_view();
    }

    #[cfg(test)]
    pub(in crate::native_app) fn set_file_view_start_from_scroll_offset(
        &mut self,
        offset_y: f32,
        row_height: f32,
    ) {
        let total_items = self.selected_audio_files().len();
        self.sample_list.prepared_window = self
            .sample_list
            .view_controller
            .set_scroll_offset_for_items(total_items, offset_y, row_height);
    }

    pub(in crate::native_app) fn apply_file_view_window_change(
        &mut self,
        change: ui::VirtualListWindowChange,
    ) {
        let viewport_rows = change.window.viewport_len().max(1);
        let overscan_rows = change
            .window
            .viewport_start
            .saturating_sub(change.window.window_start)
            .max(
                change
                    .window
                    .window_end
                    .saturating_sub(change.window.viewport_end),
            );
        self.sample_list.runtime_viewport_rows = Some(viewport_rows);
        self.sample_list
            .view_controller
            .set_total_items(change.window.total_items);
        self.sample_list
            .view_controller
            .set_viewport_len(viewport_rows);
        self.sample_list.view_controller.set_overscan(overscan_rows);
        self.sample_list.prepared_window = self
            .sample_list
            .view_controller
            .set_viewport_start(change.window.viewport_start);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn follow_selected_file_view(
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
            self.selection.selected_file.clone(),
            |file, key| file.id.as_str() == key.as_str(),
        )
        .with_context_row();
        let window = self
            .sample_list
            .view_controller
            .configure_slice_focus_changed_optional(&mut self.sample_list.follow_selection, focus);
        self.sample_list.prepared_window = window;
        window
    }

    pub(in crate::native_app) fn follow_selected_file_view_matching_tags(
        &mut self,
        viewport_rows: usize,
        overscan_rows: usize,
        guard_rows: usize,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> ui::VirtualListWindow {
        let viewport_rows = self
            .sample_list
            .runtime_viewport_rows
            .unwrap_or(viewport_rows);
        let selected_file = self.selection.selected_file.clone();
        let total_items = self.selected_audio_file_count_matching_tags(tags_by_file);
        if self.sample_list.follow_selection.focus_key() == selected_file.as_ref() {
            let projection = ui::VirtualListProjection::new(
                total_items,
                viewport_rows,
                overscan_rows,
                guard_rows,
            )
            .with_context_row();
            self.sample_list
                .view_controller
                .configure_projection(projection);
            self.sample_list.view_controller.clear_focus();
            let window = self.sample_list.view_controller.resolve();
            self.sample_list.prepared_window = window;
            return window;
        }

        let projection =
            ui::VirtualListProjection::new(total_items, viewport_rows, overscan_rows, guard_rows)
                .with_context_row();
        let focus = ui::VirtualListFocusTarget::new(
            selected_file,
            self.selected_audio_file_index_matching_tags(tags_by_file),
        );
        let window = self
            .sample_list
            .view_controller
            .configure_projection_and_focus_changed_optional(
                &mut self.sample_list.follow_selection,
                projection,
                focus,
            );
        self.sample_list.prepared_window = window;
        window
    }
}
