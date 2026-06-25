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

    pub(super) fn reconcile_file_view_after_tagged_content_change(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let total_items = self.selected_audio_file_count_matching_tags(tags_by_file);
        self.reconcile_file_view_after_content_change_with_total(total_items);
    }

    fn reconcile_file_view_after_content_change_with_total(&mut self, total_items: usize) {
        self.sample_list
            .view_controller
            .set_total_items(total_items);
        self.sample_list.view_controller.clear_focus();
        self.sample_list.prepared_window = self.sample_list.view_controller.resolve();
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
        self.sample_list.prepared_window =
            self.sample_list.view_controller.apply_window_change(change);
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
            .view_controller
            .runtime_viewport_len_or(viewport_rows);
        let selected_file = self.selection.selected_file.clone();
        let total_items = self.selected_audio_file_count_matching_tags(tags_by_file);
        let selected_index = self.selected_audio_file_index_matching_tags(tags_by_file);
        let projection =
            ui::VirtualListProjection::new(total_items, viewport_rows, overscan_rows, guard_rows)
                .with_context_row();
        let focus = ui::VirtualListFocusTarget::new(selected_file, selected_index);
        let window = if should_preserve_runtime_viewport_for_focus_change(
            &self.sample_list.follow_selection,
            &self.sample_list.view_controller,
            &focus,
        ) {
            self.sample_list
                .follow_selection
                .remember_focus_key(focus.key);
            self.sample_list
                .view_controller
                .configure_projection(projection);
            self.sample_list.view_controller.clear_focus();
            self.sample_list.view_controller.resolve()
        } else {
            self.sample_list
                .view_controller
                .configure_projection_and_focus_changed_unless_visible_optional(
                    &mut self.sample_list.follow_selection,
                    projection,
                    focus,
                )
        };
        self.sample_list.prepared_window = window;
        window
    }
}

fn should_preserve_runtime_viewport_for_focus_change(
    follow_state: &ui::VirtualListFollowState<String>,
    controller: &ui::VirtualListController,
    focus: &ui::VirtualListFocusTarget<String>,
) -> bool {
    controller.runtime_viewport_len().is_some()
        && focus.index.is_some()
        && follow_state.focus_key() != focus.key.as_ref()
}
