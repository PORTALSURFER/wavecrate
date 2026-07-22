use std::collections::HashMap;

use super::super::{FileColumnKind, FolderBrowserState, file_columns::sort_kind_for_details_sort};

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
        self.show_keyboard_focus();
        self.navigate_selected_collection(delta);
        if self.collection_keyboard_focus_active() {
            return None;
        }
        if self.selection.selected_file_active() {
            return self.navigate_selected_file(delta, extend);
        }
        self.navigate_selected_folder(delta, extend, false);
        None
    }

    pub(in crate::native_app) fn navigate_vertical_matching_tags(
        &mut self,
        delta: i32,
        extend: bool,
        preserve_folder_selection: bool,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        if delta == 0 || self.rename_active() {
            return None;
        }
        self.show_keyboard_focus();
        if self.collection_keyboard_focus_active() {
            self.navigate_selected_collection(delta);
            return None;
        }
        if self.selection.selected_file_active() {
            if self.sample_list.random_navigation.enabled {
                return self.navigate_random_matching_tags(delta, tags_by_file);
            }
            return self.navigate_selected_file_matching_tags(delta, extend, tags_by_file);
        }
        self.navigate_selected_folder(delta, extend, preserve_folder_selection);
        None
    }

    #[cfg(test)]
    pub(in crate::native_app) fn seed_random_navigation_for_tests(
        &mut self,
        result_ids: Vec<String>,
        visited: std::collections::HashSet<String>,
        history: Vec<String>,
    ) {
        self.sample_list
            .random_navigation
            .seed_for_tests(result_ids, visited, history);
    }

    pub(in crate::native_app) fn collapse_selected_folder(&mut self) -> bool {
        self.show_keyboard_focus();
        if self.rename_active() || self.selection.selected_collection.is_some() {
            return false;
        }
        if self.selected_folder_is_source_root() {
            return false;
        }
        if self.folder_has_visible_children(&self.selection.selected_folder) {
            self.tree
                .expanded_folders
                .remove(&self.selection.selected_folder)
        } else {
            false
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn expand_selected_folder(&mut self) -> bool {
        self.show_keyboard_focus();
        if self.rename_active() || self.selection.selected_collection.is_some() {
            return false;
        }
        if self.selected_folder_is_source_root() {
            return false;
        }
        if self.folder_has_visible_children(&self.selection.selected_folder) {
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
        extend: bool,
        preserve_selection: bool,
    ) -> bool {
        self.navigate_selected_folder_by_delta(delta, extend, preserve_selection)
    }

    #[cfg(not(test))]
    fn navigate_selected_folder(
        &mut self,
        delta: i32,
        extend: bool,
        preserve_selection: bool,
    ) -> bool {
        self.navigate_selected_folder_by_delta(delta, extend, preserve_selection)
    }

    fn navigate_selected_folder_by_delta(
        &mut self,
        delta: i32,
        extend: bool,
        preserve_selection: bool,
    ) -> bool {
        if delta != 0 {
            self.show_keyboard_focus();
        }
        let previous_folder_id = self.selection.selected_folder.clone();
        let folders = self.visible_folders();
        let folder_ids = folders
            .into_iter()
            .map(|folder| folder.id)
            .collect::<Vec<_>>();
        let moved = self
            .selection
            .navigate_folder(delta, extend, preserve_selection, &folder_ids)
            .is_some();
        if moved {
            self.clear_similarity_anchor_after_folder_change(&previous_folder_id);
        }
        moved
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
        if let Some(result) = self.navigate_selected_file_fast(delta, extend, tags_by_file) {
            return result;
        }
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        self.navigate_selected_file_in_ids(delta, extend, &file_ids)
    }

    pub(in crate::native_app) fn navigate_selected_file_in_ids(
        &mut self,
        delta: i32,
        extend: bool,
        file_ids: &[String],
    ) -> Option<String> {
        if delta != 0 {
            self.show_keyboard_focus();
        }
        self.selection.navigate_file(delta, extend, file_ids)
    }

    fn navigate_random_matching_tags(
        &mut self,
        delta: i32,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        let file_ids = if self.curation_mode_enabled() {
            self.selected_curation_bucket_file_ids_matching_tags(tags_by_file)
        } else {
            self.selected_audio_file_ids_matching_tags(tags_by_file)
        };
        if delta < 0 {
            self.sample_list
                .random_navigation
                .reconcile(self.selection.selected_file_id(), &file_ids);
            let target = self.sample_list.random_navigation.previous()?;
            self.selection.select_single_file(target.clone(), &file_ids);
            return Some(target);
        }

        let target = self
            .sample_list
            .random_navigation
            .next(self.selection.selected_file_id(), &file_ids)?;
        self.selection.select_single_file(target.clone(), &file_ids);
        Some(target)
    }

    pub(in crate::native_app) fn selected_audio_file_index_fast_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<Option<usize>> {
        if !self.fast_untagged_navigation_available(tags_by_file) {
            return None;
        }

        let selected = self.selection.selected_file_id()?;
        Some(self.selected_audio_file_index_without_tag_filter(selected))
    }

    fn navigate_selected_file_fast(
        &mut self,
        delta: i32,
        extend: bool,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<Option<String>> {
        if !self.fast_untagged_navigation_available(tags_by_file) || extend {
            return None;
        }

        let selected = self.selection.selected_file_id()?.to_owned();
        let target = self.neighboring_selected_audio_file_id_without_tag_filter(&selected, delta);
        Some(target.and_then(|target| self.selection.navigate_file_to_adjacent_visible_id(target)))
    }

    fn fast_untagged_navigation_available(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> bool {
        self.active_listing_reveal_id(Some(tags_by_file)).is_none()
            && self.active_harvest_filter().is_none()
            && !tag_filter_has_requirements(&self.filters.tag_filter)
            && !self.folder_subtree_listing_enabled()
            && !self.filters.playback_type_enabled
            && self.filters.playback_type_filter.is_empty()
            && !self.filters.curation.enabled
            && sort_kind_for_details_sort(&self.sample_list.file_sort)
                != FileColumnKind::PlaybackType
    }
}

fn tag_filter_has_requirements(tag_filter: &str) -> bool {
    tag_filter.split(',').any(|tag| !tag.trim().is_empty())
}
