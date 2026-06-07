use radiant::{prelude as ui, widgets::PointerModifiers};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use wavecrate::sample_sources::Rating;

use super::{
    FolderBrowserState, FolderVerifyResult,
    path_helpers::path_id,
    scanning::{file_entry, load_folder_at_path, load_root_folder, upsert_file, upsert_folder},
};

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

        let source_changed = self.selected_source != source_id;
        self.cancel_rename();
        self.selected_collection = None;
        self.collection_rename_edit = None;
        self.selected_source = source_id;
        self.selected_folder = parent_id;
        self.selected_file = Some(file_id.clone());
        self.selected_file_ids.clear();
        self.selected_file_ids.insert(file_id);
        self.selected_file_ids_explicit = false;
        self.reset_file_view();
        self.folders = vec![root_folder];
        self.prewarm_selected_source_audio_projection_cache();
        if source_changed {
            self.expanded_folders.clear();
        }
        self.expanded_folders
            .extend(folder_ancestor_ids(&source_root, parent));
        true
    }

    fn find_loaded_source_containing_file(
        &self,
        path: &Path,
        parent: &Path,
        file_id: &str,
    ) -> Option<(String, PathBuf, super::FolderEntry)> {
        self.sources.iter().find_map(|source| {
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
            .sources
            .iter()
            .enumerate()
            .filter(|(_, source)| path.starts_with(&source.root))
            .max_by_key(|(_, source)| source.root.components().count())
            .map(|(index, _)| index)
        else {
            return;
        };
        if self.sources[index].root_folder.is_none() {
            let root = self.sources[index].root.clone();
            self.sources[index].root_folder = Some(load_root_folder(root));
            self.sources[index].loading_task = None;
            self.bump_file_content_revision();
        }
    }

    pub(in crate::native_app) fn selected_file_paths(&self) -> Vec<PathBuf> {
        let selected = self.active_selected_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .map(|file| PathBuf::from(&file.id))
            .collect()
    }

    pub(in crate::native_app) fn first_audio_file_path(&self) -> Option<PathBuf> {
        self.sources
            .iter()
            .find(|source| source.id == self.selected_source)
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
        let selected = self.active_selected_file_ids();
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
        let selected = self.active_selected_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .count()
    }

    fn active_selected_file_ids(&self) -> HashSet<String> {
        if self.selected_file_ids_explicit || !self.selected_file_ids.is_empty() {
            return self.selected_file_ids.clone();
        }
        self.selected_file
            .as_deref()
            .map(|id| [id.to_string()].into_iter().collect())
            .unwrap_or_default()
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
        if self.selected_collection.is_some() && self.selected_file.is_none() {
            return self.navigate_into_active_file_list(delta);
        }
        if self.selected_file.is_some() {
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
        if self.selected_collection.is_some() && self.selected_file.is_none() {
            return self.navigate_into_active_file_list_matching_tags(delta, tags_by_file);
        }
        if self.selected_file.is_some() {
            return self.navigate_selected_file_matching_tags(delta, extend, tags_by_file);
        }
        self.navigate_selected_folder(delta);
        None
    }

    pub(in crate::native_app) fn collapse_selected_folder(&mut self) -> bool {
        if self.rename_active() || self.selected_collection.is_some() {
            return false;
        }
        if self.selected_folder_is_source_root() {
            return false;
        }
        if self.folder_has_children(&self.selected_folder) {
            self.expanded_folders.remove(&self.selected_folder)
        } else {
            false
        }
    }

    pub(in crate::native_app) fn expand_selected_folder(&mut self) -> bool {
        if self.rename_active() || self.selected_collection.is_some() {
            return false;
        }
        if self.selected_folder_is_source_root() {
            return false;
        }
        if self.folder_has_children(&self.selected_folder) {
            self.expanded_folders.insert(self.selected_folder.clone())
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
            .position(|folder| folder.id == self.selected_folder)
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
        let mut selection = self.file_selection_model();
        let target = if extend {
            selection.navigate_preserving_existing(delta as isize, file_ids)?
        } else {
            selection.navigate(delta as isize, file_ids, false)?
        };
        self.apply_file_selection_model(selection);
        self.selected_file_ids_explicit = extend;
        Some(target)
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
            let mut selection = ui::KeyedListSelection::new();
            selection.select_with_intent(id, &file_ids, ui::ListSelectionIntent::Replace);
            self.apply_file_selection_model(selection);
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

        let mut selection = self.file_selection_model();
        selection.select_with_intent(
            id,
            &file_ids,
            ui::ListSelectionIntent::from_extend_toggle(modifiers.shift, modifiers.command),
        );
        self.apply_file_selection_model(selection);
        self.selected_file_ids_explicit = modifiers.shift || modifiers.command;
    }

    pub(in crate::native_app) fn focus_file_preserving_selection(&mut self, id: String) {
        if self.selected_file_ids.contains(&id)
            && self.selected_audio_files().iter().any(|file| file.id == id)
        {
            self.selected_file = Some(id);
        } else {
            self.select_file(id);
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn select_all_audio_files(&mut self) -> usize {
        if self.rename_active() {
            return self.selected_file_ids.len();
        }
        let ids = self.selected_audio_file_ids();
        self.select_audio_file_ids(ids)
    }

    pub(in crate::native_app) fn select_all_audio_files_matching_tags(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> usize {
        if self.rename_active() {
            return self.selected_file_ids.len();
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
        if self.selected_collection.is_some() && self.selected_file.is_none() {
            self.navigate_into_active_file_list_matching_tags(1, tags_by_file)?;
        }
        let file_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        let focused = self.selected_file.as_ref()?;
        let current_index = file_ids.iter().position(|id| id == focused)?;
        let toggled_id = focused.clone();
        let already_marked =
            self.selected_file_ids_explicit && self.selected_file_ids.contains(&toggled_id);
        let toggled_selected = if already_marked {
            self.selected_file_ids.remove(&toggled_id);
            false
        } else {
            self.selected_file_ids.insert(toggled_id.clone());
            true
        };
        self.selected_file_ids_explicit = true;
        let focused_id = file_ids[current_index.saturating_add(1).min(file_ids.len() - 1)].clone();
        self.selected_file = Some(focused_id.clone());
        Some(ToggleSelectedSampleAdvanceResult {
            toggled_id,
            toggled_selected,
            focused_id,
        })
    }

    fn select_audio_file_ids(&mut self, ids: Vec<String>) -> usize {
        let mut selection = self.file_selection_model();
        selection.select_all(&ids);
        self.apply_file_selection_model(selection);
        self.selected_file_ids_explicit = true;
        self.selected_file_ids.len()
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

    fn file_selection_model(&self) -> ui::KeyedListSelection<String> {
        ui::KeyedListSelection::from_parts(
            self.selected_file.clone(),
            self.selected_file.clone(),
            self.selected_file_ids.clone(),
        )
    }

    fn apply_file_selection_model(&mut self, selection: ui::KeyedListSelection<String>) {
        self.selected_file = selection.focused_key().cloned();
        self.selected_file_ids = selection.selected_keys().iter().cloned().collect();
        self.selected_file_ids_explicit = false;
    }

    pub(in crate::native_app) fn refresh_file_path(&mut self, path: &Path) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let parent_id = path_id(parent);
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return false;
        };
        let Some(root_folder) = &mut source.root_folder else {
            return false;
        };
        let Some(parent_folder) = root_folder.find_mut(&parent_id) else {
            return false;
        };
        upsert_file(&mut parent_folder.files, file_entry(&path.to_path_buf()));
        self.folders = vec![root_folder.clone()];
        self.bump_file_content_revision();
        true
    }

    pub(in crate::native_app) fn refresh_filesystem_paths(
        &mut self,
        source_id: &str,
        relative_paths: &[PathBuf],
    ) -> bool {
        let Some(source_index) = self
            .sources
            .iter()
            .position(|source| source.id == source_id)
        else {
            return false;
        };
        let root = self.sources[source_index].root.clone();
        let mut changed = false;
        for relative_path in relative_paths {
            changed |= self.refresh_one_source_relative_path(source_index, &root, relative_path);
        }
        if changed {
            self.after_source_tree_changed(source_id);
        }
        changed
    }

    pub(in crate::native_app) fn apply_direct_folder_verify_result(
        &mut self,
        result: FolderVerifyResult,
    ) -> bool {
        let Some(snapshot) = result.snapshot else {
            return false;
        };
        let Some(source_index) = self
            .sources
            .iter()
            .position(|source| source.id == result.source_id)
        else {
            return false;
        };
        let folder_id = path_id(&result.folder_path);
        let Some(root_folder) = self.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        let Some(folder) = root_folder.find_mut(&folder_id) else {
            return false;
        };
        if !folder.replace_direct_entries(snapshot.child_paths, snapshot.files) {
            return false;
        }
        if self.selected_source == result.source_id {
            self.folders = vec![root_folder.clone()];
            if self.selected_folder == folder_id {
                let visible_ids = self
                    .selected_audio_files()
                    .into_iter()
                    .map(|file| file.id.clone())
                    .collect::<HashSet<_>>();
                self.selected_file_ids.retain(|id| visible_ids.contains(id));
                if self
                    .selected_file
                    .as_ref()
                    .is_some_and(|id| !visible_ids.contains(id))
                {
                    self.selected_file = None;
                }
            }
        }
        self.bump_file_content_revision();
        true
    }

    fn refresh_one_source_relative_path(
        &mut self,
        source_index: usize,
        source_root: &Path,
        relative_path: &Path,
    ) -> bool {
        let absolute_path = source_root.join(relative_path);
        if absolute_path.is_dir() {
            return self.refresh_existing_folder_path(source_index, source_root, &absolute_path);
        }
        if absolute_path.is_file() {
            return self.refresh_existing_file_path(source_index, &absolute_path);
        }
        self.remove_missing_path_from_source(source_index, &absolute_path)
    }

    fn refresh_existing_file_path(&mut self, source_index: usize, path: &Path) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let parent_id = path_id(parent);
        let Some(root_folder) = self.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        if root_folder.find(&parent_id).is_none() {
            let source_root = self.sources[source_index].root.clone();
            return self.refresh_existing_folder_path(source_index, &source_root, parent);
        }
        let Some(root_folder) = self.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        let Some(parent_folder) = root_folder.find_mut(&parent_id) else {
            return false;
        };
        upsert_file(&mut parent_folder.files, file_entry(&path.to_path_buf()))
    }

    fn refresh_existing_folder_path(
        &mut self,
        source_index: usize,
        source_root: &Path,
        path: &Path,
    ) -> bool {
        let Some(folder) = load_folder_at_path(path, source_root) else {
            return false;
        };
        let Some(root_folder) = self.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        if root_folder.id == folder.id {
            if *root_folder == folder {
                return false;
            }
            *root_folder = folder;
            return true;
        }
        let Some(parent) = path.parent() else {
            return false;
        };
        let Some(parent_folder) = root_folder.find_mut(&path_id(parent)) else {
            return false;
        };
        upsert_folder(&mut parent_folder.children, folder)
    }

    fn remove_missing_path_from_source(&mut self, source_index: usize, path: &Path) -> bool {
        let path_id = path_id(path);
        let Some(root_folder) = self.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        let removed_folder = root_folder.remove_child_by_id(&path_id);
        let removed_file = root_folder.remove_file_by_id(&path_id);
        removed_folder || removed_file
    }

    fn after_source_tree_changed(&mut self, source_id: &str) {
        if let Some(root_folder) = self
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .and_then(|source| source.root_folder.clone())
        {
            if self.selected_source == source_id {
                if root_folder.find(&self.selected_folder).is_none() {
                    self.selected_folder = root_folder.id.clone();
                    self.selected_file = None;
                    self.selected_file_ids.clear();
                    self.selected_file_ids_explicit = false;
                }
                self.folders = vec![root_folder];
            }
            self.bump_file_content_revision();
        }
    }

    pub(in crate::native_app) fn set_file_rating_state(
        &mut self,
        path: &Path,
        rating: Rating,
        locked: bool,
    ) -> bool {
        let file_id = path_id(path);
        let mut changed = false;
        for source in &mut self.sources {
            let Some(root_folder) = &mut source.root_folder else {
                continue;
            };
            changed |= root_folder.set_file_rating(&file_id, rating, locked);
            if source.id == self.selected_source {
                self.folders = vec![root_folder.clone()];
            }
        }
        if changed {
            self.bump_file_content_revision();
        }
        changed
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
