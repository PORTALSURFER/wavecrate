use radiant::{prelude as ui, widgets::PointerModifiers};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use wavecrate::sample_sources::Rating;

use super::{
    FolderBrowserState,
    path_helpers::path_id,
    scanning::{file_entry, load_root_folder, upsert_file},
};

impl FolderBrowserState {
    pub(in crate::gui_app) fn focus_file_across_sources(&mut self, path: &Path) -> bool {
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
        self.reset_file_view();
        self.folders = vec![root_folder];
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
        }
    }

    pub(in crate::gui_app) fn selected_file_paths(&self) -> Vec<PathBuf> {
        let selected = self.active_selected_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .map(|file| PathBuf::from(&file.id))
            .collect()
    }

    pub(in crate::gui_app) fn first_audio_file_path(&self) -> Option<PathBuf> {
        self.sources
            .iter()
            .find(|source| source.id == self.selected_source)
            .and_then(|source| source.root_folder.as_ref())
            .and_then(first_audio_file_in_folder)
            .map(PathBuf::from)
    }

    pub(in crate::gui_app) fn selected_file_rating_candidates(
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

    pub(in crate::gui_app) fn selected_audio_file_count(&self) -> usize {
        let selected = self.active_selected_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .count()
    }

    fn active_selected_file_ids(&self) -> HashSet<String> {
        if !self.selected_file_ids.is_empty() {
            return self.selected_file_ids.clone();
        }
        self.selected_file
            .as_deref()
            .map(|id| [id.to_string()].into_iter().collect())
            .unwrap_or_default()
    }

    pub(in crate::gui_app) fn navigate_vertical(
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

    pub(in crate::gui_app) fn collapse_selected_folder(&mut self) -> bool {
        if self.rename_active() || self.selected_collection.is_some() {
            return false;
        }
        if self.folder_has_children(&self.selected_folder) {
            self.expanded_folders.remove(&self.selected_folder)
        } else {
            false
        }
    }

    pub(in crate::gui_app) fn expand_selected_folder(&mut self) -> bool {
        if self.rename_active() || self.selected_collection.is_some() {
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

    fn navigate_selected_file(&mut self, delta: i32, extend: bool) -> Option<String> {
        let file_ids = self.selected_audio_file_ids();
        let mut selection = self.file_selection_model();
        let target = if extend {
            selection.navigate_preserving_existing(delta as isize, &file_ids)?
        } else {
            selection.navigate(delta as isize, &file_ids, false)?
        };
        self.apply_file_selection_model(selection);
        Some(target)
    }

    /// Selects the first reachable file when collection mode owns navigation focus.
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

    pub(in crate::gui_app) fn select_file(&mut self, id: String) {
        let file_ids = self.selected_audio_file_ids();
        if file_ids.contains(&id) {
            self.cancel_rename();
            let mut selection = ui::KeyedListSelection::new();
            selection.select_with_intent(id, &file_ids, ui::ListSelectionIntent::Replace);
            self.apply_file_selection_model(selection);
        }
    }

    pub(in crate::gui_app) fn select_file_with_modifiers(
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
    }

    pub(in crate::gui_app) fn focus_file_preserving_selection(&mut self, id: String) {
        if self.selected_file_ids.contains(&id)
            && self.selected_audio_files().iter().any(|file| file.id == id)
        {
            self.selected_file = Some(id);
        } else {
            self.select_file(id);
        }
    }

    pub(in crate::gui_app) fn select_all_audio_files(&mut self) -> usize {
        if self.rename_active() {
            return self.selected_file_ids.len();
        }
        let ids = self.selected_audio_file_ids();
        let mut selection = self.file_selection_model();
        selection.select_all(&ids);
        self.apply_file_selection_model(selection);
        self.selected_file_ids.len()
    }

    fn selected_audio_file_ids(&self) -> Vec<String> {
        self.selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect()
    }

    fn file_selection_model(&self) -> ui::KeyedListSelection<String> {
        ui::KeyedListSelection::from_parts(
            self.selected_file.clone(),
            self.selected_file.clone(),
            self.active_selected_file_ids(),
        )
    }

    fn apply_file_selection_model(&mut self, selection: ui::KeyedListSelection<String>) {
        self.selected_file = selection.focused_key().cloned();
        self.selected_file_ids = selection.selected_keys().iter().cloned().collect();
    }

    pub(in crate::gui_app) fn refresh_file_path(&mut self, path: &Path) -> bool {
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
        true
    }

    pub(in crate::gui_app) fn set_file_rating_state(
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
        changed
    }
}

fn first_audio_file_in_folder(folder: &super::FolderEntry) -> Option<&str> {
    if let Some(file) = folder.files.iter().find(|file| file.is_audio()) {
        return Some(file.id.as_str());
    }
    folder.children.iter().find_map(first_audio_file_in_folder)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct SelectedFileRatingCandidate {
    pub(in crate::gui_app) path: PathBuf,
    pub(in crate::gui_app) rating: Rating,
    pub(in crate::gui_app) locked: bool,
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
