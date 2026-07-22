use std::collections::HashMap;

use radiant::widgets::PointerModifiers;

use super::super::FolderBrowserState;
use super::ToggleSelectedSampleResult;

impl FolderBrowserState {
    pub(in crate::native_app) fn select_file(&mut self, id: String) {
        let file_ids = self.selected_audio_file_ids();
        if file_ids.contains(&id) {
            self.cancel_rename();
            self.clear_source_keyboard_focus();
            self.hide_keyboard_focus();
            self.selection.select_single_file(id, &file_ids);
        }
    }

    #[cfg(test)]
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
        self.clear_source_keyboard_focus();
        self.hide_keyboard_focus();
        self.selection
            .select_file_with_modifiers(id, &file_ids, modifiers);
    }

    pub(in crate::native_app) fn select_file_with_modifiers_matching_tags(
        &mut self,
        id: String,
        modifiers: PointerModifiers,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        if self.rename_active() || !file_ids.contains(&id) {
            return;
        }
        self.cancel_rename();
        self.clear_source_keyboard_focus();
        self.hide_keyboard_focus();
        self.selection
            .select_file_with_modifiers(id, &file_ids, modifiers);
    }

    pub(in crate::native_app) fn select_known_starmap_file_for_audition(&mut self, id: String) {
        if self.rename_active() {
            return;
        }
        self.cancel_rename();
        if self.selection.select_known_single_file(id) {
            self.clear_source_keyboard_focus();
        }
    }

    pub(in crate::native_app) fn focus_file_preserving_selection(&mut self, id: String) {
        let file_ids = self.selected_audio_file_ids();
        if self
            .selection
            .focus_file_preserving_selection(id, &file_ids)
        {
            self.clear_source_keyboard_focus();
        }
    }

    pub(in crate::native_app) fn focus_file_preserving_selection_matching_tags(
        &mut self,
        id: String,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        if self
            .selection
            .focus_file_preserving_selection(id, &file_ids)
        {
            self.clear_source_keyboard_focus();
        }
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

    pub(in crate::native_app) fn toggle_focused_sample_selection(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<ToggleSelectedSampleResult> {
        self.show_keyboard_focus();
        if self.rename_active() {
            return None;
        }
        if self
            .selection
            .selected_collection_active_without_file_focus()
        {
            self.focus_first_active_collection_file_matching_tags(tags_by_file)?;
        }
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        let outcome = self.selection.toggle_focused_file(&file_ids)?;
        Some(ToggleSelectedSampleResult {
            toggled_id: outcome.toggled_id,
            toggled_selected: outcome.toggled_selected,
            focused_id: outcome.focused_id,
        })
    }

    fn select_audio_file_ids(&mut self, ids: Vec<String>) -> usize {
        let count = self.selection.select_all_files(ids);
        if count > 0 {
            self.clear_source_keyboard_focus();
        }
        count
    }

    fn focus_first_active_collection_file_matching_tags(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        let target = file_ids.first()?.clone();
        self.selection.select_single_file(target.clone(), &file_ids);
        Some(target)
    }
}
