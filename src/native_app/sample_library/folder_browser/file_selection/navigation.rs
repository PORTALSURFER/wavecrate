use radiant::prelude as ui;
use std::collections::HashMap;

use super::super::FolderBrowserState;

impl FolderBrowserState {
    #[cfg(test)]
    pub(in crate::native_app) fn navigate_vertical(
        &mut self,
        delta: i32,
        extend: bool,
    ) -> Option<String> {
        if delta == 0 || self.rename_active() {
            return None;
        }
        if self
            .selection
            .selected_collection_active_without_file_focus()
        {
            return self.navigate_into_active_file_list(delta);
        }
        if self.selection.selected_file_active() {
            return self.navigate_selected_file(delta, extend);
        }
        self.navigate_selected_folder(delta);
        None
    }

    pub(in crate::native_app) fn navigate_vertical_matching_tags(
        &mut self,
        delta: i32,
        extend: bool,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        if delta == 0 || self.rename_active() {
            return None;
        }
        if self
            .selection
            .selected_collection_active_without_file_focus()
        {
            return self.navigate_into_active_file_list_matching_tags(delta, tags_by_file);
        }
        if self.selection.selected_file_active() {
            return self.navigate_selected_file_matching_tags(delta, extend, tags_by_file);
        }
        self.navigate_selected_folder(delta);
        None
    }

    pub(in crate::native_app) fn collapse_selected_folder(&mut self) -> bool {
        if self.rename_active() || self.selection.selected_collection.is_some() {
            return false;
        }
        if self.selected_folder_is_source_root() {
            return false;
        }
        if self.folder_has_children(&self.selection.selected_folder) {
            self.tree
                .expanded_folders
                .remove(&self.selection.selected_folder)
        } else {
            false
        }
    }

    pub(in crate::native_app) fn expand_selected_folder(&mut self) -> bool {
        if self.rename_active() || self.selection.selected_collection.is_some() {
            return false;
        }
        if self.selected_folder_is_source_root() {
            return false;
        }
        if self.folder_has_children(&self.selection.selected_folder) {
            self.tree
                .expanded_folders
                .insert(self.selection.selected_folder.clone())
        } else {
            false
        }
    }

    #[cfg(test)]
    pub(in crate::native_app::sample_library::folder_browser) fn navigate_selected_folder(
        &mut self,
        delta: i32,
    ) -> bool {
        self.navigate_selected_folder_by_delta(delta)
    }

    #[cfg(not(test))]
    fn navigate_selected_folder(&mut self, delta: i32) -> bool {
        self.navigate_selected_folder_by_delta(delta)
    }

    fn navigate_selected_folder_by_delta(&mut self, delta: i32) -> bool {
        let folders = self.visible_folders();
        let Some(current_index) = folders
            .iter()
            .position(|folder| folder.id == self.selection.selected_folder)
        else {
            return false;
        };
        let target_index = ui::list_index_after_delta(current_index, delta as isize, folders.len())
            .unwrap_or(current_index);
        if target_index == current_index {
            return false;
        }
        self.select_folder(folders[target_index].id.clone());
        true
    }

    #[cfg(test)]
    fn navigate_selected_file(&mut self, delta: i32, extend: bool) -> Option<String> {
        let file_ids = self.selected_audio_file_ids();
        self.navigate_selected_file_in_ids(delta, extend, &file_ids)
    }

    fn navigate_selected_file_matching_tags(
        &mut self,
        delta: i32,
        extend: bool,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        self.navigate_selected_file_in_ids(delta, extend, &file_ids)
    }

    fn navigate_selected_file_in_ids(
        &mut self,
        delta: i32,
        extend: bool,
        file_ids: &[String],
    ) -> Option<String> {
        self.selection.navigate_file(delta, extend, file_ids)
    }

    /// Selects the first reachable file when collection mode owns navigation focus.
    #[cfg(test)]
    fn navigate_into_active_file_list(&mut self, delta: i32) -> Option<String> {
        let file_ids = self.selected_audio_file_ids();
        let target = if delta < 0 {
            file_ids.last()
        } else {
            file_ids.first()
        }?
        .clone();
        self.select_file(target.clone());
        Some(target)
    }

    pub(in crate::native_app::sample_library::folder_browser) fn navigate_into_active_file_list_matching_tags(
        &mut self,
        delta: i32,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        let target = if delta < 0 {
            file_ids.last()
        } else {
            file_ids.first()
        }?
        .clone();
        self.select_file(target.clone());
        Some(target)
    }
}
