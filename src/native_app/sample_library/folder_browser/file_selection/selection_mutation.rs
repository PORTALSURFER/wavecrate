use std::collections::HashMap;

use radiant::widgets::PointerModifiers;

use super::super::FolderBrowserState;
use super::ToggleSelectedSampleAdvanceResult;

impl FolderBrowserState {
    pub(in crate::native_app) fn select_file(&mut self, id: String) {
        let file_ids = self.selected_audio_file_ids();
        if file_ids.contains(&id) {
            self.cancel_rename();
            self.selection.select_single_file(id, &file_ids);
        }
    }

    pub(in crate::native_app) fn select_file_with_modifiers(
        &mut self,
        id: String,
        modifiers: PointerModifiers,
    ) {
        let file_ids = self.selected_audio_file_ids();
        if self.rename_active() || !file_ids.contains(&id) {
            return;
        }
        self.cancel_rename();
        self.selection
            .select_file_with_modifiers(id, &file_ids, modifiers);
    }

    pub(in crate::native_app) fn focus_file_preserving_selection(&mut self, id: String) {
        let file_ids = self.selected_audio_file_ids();
        self.selection
            .focus_file_preserving_selection(id, &file_ids);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn select_all_audio_files(&mut self) -> usize {
        if self.rename_active() {
            return self.selection.selected_file_count();
        }
        let ids = self.selected_audio_file_ids();
        self.select_audio_file_ids(ids)
    }

    pub(in crate::native_app) fn select_all_audio_files_matching_tags(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> usize {
        if self.rename_active() {
            return self.selection.selected_file_count();
        }
        let ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        self.select_audio_file_ids(ids)
    }

    pub(in crate::native_app) fn toggle_focused_sample_selection_and_advance(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<ToggleSelectedSampleAdvanceResult> {
        if self.rename_active() {
            return None;
        }
        if self
            .selection
            .selected_collection_active_without_file_focus()
        {
            self.navigate_into_active_file_list_matching_tags(1, tags_by_file)?;
        }
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        let outcome = self.selection.toggle_focused_file_and_advance(&file_ids)?;
        Some(ToggleSelectedSampleAdvanceResult {
            toggled_id: outcome.toggled_id,
            toggled_selected: outcome.toggled_selected,
            focused_id: outcome.focused_id,
        })
    }

    fn select_audio_file_ids(&mut self, ids: Vec<String>) -> usize {
        self.selection.select_all_files(ids)
    }
}
