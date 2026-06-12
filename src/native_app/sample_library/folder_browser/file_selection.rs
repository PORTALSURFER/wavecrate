use radiant::{prelude as ui, widgets::PointerModifiers};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wavecrate::sample_sources::Rating;

use super::{FolderBrowserState, path_helpers::path_id, scanning::load_root_folder};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct ToggleSelectedSampleAdvanceResult {
    pub(in crate::native_app) toggled_id: String,
    pub(in crate::native_app) toggled_selected: bool,
    pub(in crate::native_app) focused_id: String,
}

impl FolderBrowserState {
    pub(in crate::native_app) fn focus_file_across_sources(&mut self, path: &Path) -> bool {
        self.ensure_loaded_source_containing_path(path);
        let file_id = path_id(path);
        let Some(parent) = path.parent() else {
            return false;
        };
        let parent_id = path_id(parent);
        let Some((source_id, source_root, root_folder)) =
            self.find_loaded_source_containing_file(path, parent, &file_id)
        else {
            return false;
        };

        let source_changed = self.source.selected_source != source_id;
        self.cancel_rename();
        self.selection.selected_collection = None;
        self.selection.folder_before_collection = None;
        self.collection_panel.rename_edit = None;
        self.source.selected_source = source_id;
        self.selection.selected_folder = parent_id;
        self.selection.set_focus_file_set(file_id);
        self.reset_file_view();
        self.tree.folders = vec![root_folder];
        self.prewarm_selected_source_audio_projection_cache();
        if source_changed {
            self.tree.expanded_folders.clear();
        }
        self.tree
            .expanded_folders
            .extend(folder_ancestor_ids(&source_root, parent));
        true
    }

    fn find_loaded_source_containing_file(
        &self,
        path: &Path,
        parent: &Path,
        file_id: &str,
    ) -> Option<(String, PathBuf, super::FolderEntry)> {
        self.source.sources.iter().find_map(|source| {
            if !path.starts_with(&source.root) {
                return None;
            }
            let root_folder = source.root_folder.as_ref()?;
            let parent_folder = root_folder.find(&path_id(parent))?;
            parent_folder
                .files
                .iter()
                .any(|file| file.id == file_id && file.is_audio())
                .then(|| (source.id.clone(), source.root.clone(), root_folder.clone()))
        })
    }

    fn ensure_loaded_source_containing_path(&mut self, path: &Path) {
        let Some(index) = self
            .source
            .sources
            .iter()
            .enumerate()
            .filter(|(_, source)| path.starts_with(&source.root))
            .max_by_key(|(_, source)| source.root.components().count())
            .map(|(index, _)| index)
        else {
            return;
        };
        if self.source.sources[index].root_folder.is_none() {
            let root = self.source.sources[index].root.clone();
            self.source.sources[index].root_folder = Some(load_root_folder(root));
            self.source.sources[index].loading_task = None;
            self.bump_file_content_revision();
        }
    }

    pub(in crate::native_app) fn selected_file_paths(&self) -> Vec<PathBuf> {
        let selected = self.selection.active_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .map(|file| PathBuf::from(&file.id))
            .collect()
    }

    pub(in crate::native_app) fn first_audio_file_path(&self) -> Option<PathBuf> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)
            .and_then(|source| source.root_folder.as_ref())
            .and_then(first_audio_file_in_folder)
            .map(PathBuf::from)
    }

    pub(in crate::native_app) fn random_playback_available(&self) -> bool {
        !self.selected_audio_file_ids().is_empty()
            || self
                .selected_source_audio_files()
                .into_iter()
                .next()
                .is_some()
    }

    pub(in crate::native_app) fn random_playback_candidate(&self, unit: f32) -> Option<String> {
        let visible_files = self.selected_audio_file_ids();
        if let Some(candidate) = random_candidate_from_ids(&visible_files, unit) {
            return Some(candidate);
        }

        let source_files = self
            .selected_source_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        random_candidate_from_ids(&source_files, unit)
    }

    pub(in crate::native_app) fn selected_file_rating_candidates(
        &self,
    ) -> Vec<SelectedFileRatingCandidate> {
        let selected = self.selection.active_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .map(|file| SelectedFileRatingCandidate {
                path: PathBuf::from(&file.id),
                rating: file.rating,
                locked: file.rating_locked,
            })
            .collect()
    }

    pub(in crate::native_app) fn selected_audio_file_count(&self) -> usize {
        let selected = self.selection.active_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .count()
    }

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
    pub(super) fn navigate_selected_folder(&mut self, delta: i32) -> bool {
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

    fn navigate_into_active_file_list_matching_tags(
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

    fn selected_audio_file_ids(&self) -> Vec<String> {
        self.selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect()
    }

    fn selected_audio_file_ids_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        self.selected_audio_files_matching_tags(tags_by_file)
            .into_iter()
            .map(|file| file.id.clone())
            .collect()
    }
}

fn random_candidate_from_ids(file_ids: &[String], unit: f32) -> Option<String> {
    let index = ui::unit_interval_index(unit, file_ids.len())?;
    Some(file_ids[index].clone())
}

fn first_audio_file_in_folder(folder: &super::FolderEntry) -> Option<&str> {
    if let Some(file) = folder.files.iter().find(|file| file.is_audio()) {
        return Some(file.id.as_str());
    }
    folder.children.iter().find_map(first_audio_file_in_folder)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SelectedFileRatingCandidate {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) rating: Rating,
    pub(in crate::native_app) locked: bool,
}

fn folder_ancestor_ids(root: &Path, folder: &Path) -> Vec<String> {
    let mut ids = vec![path_id(root)];
    let Ok(relative) = folder.strip_prefix(root) else {
        return ids;
    };
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        ids.push(path_id(&current));
    }
    ids
}
