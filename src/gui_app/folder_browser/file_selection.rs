use radiant::widgets::PointerModifiers;
use std::path::{Path, PathBuf};

use super::{
    FolderBrowserState,
    path_helpers::{offset_index, path_id},
    scanning::{file_entry, upsert_file},
};

impl FolderBrowserState {
    pub(in crate::gui_app) fn selected_file_paths(&self) -> Vec<PathBuf> {
        let selected = if self.selected_file_ids.is_empty() {
            self.selected_file
                .as_deref()
                .map(|id| [id.to_string()].into_iter().collect())
                .unwrap_or_default()
        } else {
            self.selected_file_ids.clone()
        };
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .map(|file| PathBuf::from(&file.id))
            .collect()
    }

    pub(in crate::gui_app) fn selected_audio_file_count(&self) -> usize {
        let selected = if self.selected_file_ids.is_empty() {
            self.selected_file
                .as_deref()
                .map(|id| [id.to_string()].into_iter().collect())
                .unwrap_or_default()
        } else {
            self.selected_file_ids.clone()
        };
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .count()
    }

    pub(in crate::gui_app) fn navigate_vertical(
        &mut self,
        delta: i32,
        extend: bool,
    ) -> Option<String> {
        if delta == 0 || self.rename_active() {
            return None;
        }
        if self.selected_file.is_some() {
            return self.navigate_selected_file(delta, extend);
        }
        self.navigate_selected_folder(delta);
        None
    }

    pub(in crate::gui_app) fn collapse_selected_folder(&mut self) -> bool {
        if self.rename_active() {
            return false;
        }
        if self.folder_has_children(&self.selected_folder) {
            self.expanded_folders.remove(&self.selected_folder)
        } else {
            false
        }
    }

    pub(in crate::gui_app) fn expand_selected_folder(&mut self) -> bool {
        if self.rename_active() {
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
        let target_index = offset_index(current_index, delta, folders.len());
        if target_index == current_index {
            return false;
        }
        self.select_folder(folders[target_index].id.clone());
        true
    }

    fn navigate_selected_file(&mut self, delta: i32, extend: bool) -> Option<String> {
        let files = self.selected_audio_files();
        let current = self.selected_file.as_deref()?;
        let current_index = files.iter().position(|file| file.id == current)?;
        let target_index = offset_index(current_index, delta, files.len());
        if target_index == current_index {
            return None;
        }
        let target = files[target_index].id.clone();
        if extend {
            self.selected_file_ids.insert(current.to_string());
            self.selected_file_ids.insert(target.clone());
        } else {
            self.selected_file_ids.clear();
            self.selected_file_ids.insert(target.clone());
        }
        self.selected_file = Some(target.clone());
        Some(target)
    }

    pub(in crate::gui_app) fn select_file(&mut self, id: String) {
        if self.selected_files().iter().any(|file| file.id == id) {
            self.cancel_rename();
            self.selected_file = Some(id.clone());
            self.selected_file_ids.clear();
            self.selected_file_ids.insert(id);
        }
    }

    pub(in crate::gui_app) fn select_file_with_modifiers(
        &mut self,
        id: String,
        modifiers: PointerModifiers,
    ) {
        if self.rename_active() || !self.selected_files().iter().any(|file| file.id == id) {
            return;
        }
        self.cancel_rename();

        if modifiers.shift {
            self.select_file_range_to(id, modifiers.command);
            return;
        }

        if modifiers.command {
            self.toggle_file_selection(id);
            return;
        }

        self.selected_file = Some(id.clone());
        self.selected_file_ids.clear();
        self.selected_file_ids.insert(id);
    }

    pub(in crate::gui_app) fn focus_file_preserving_selection(&mut self, id: String) {
        if self.selected_file_ids.contains(&id)
            && self.selected_files().iter().any(|file| file.id == id)
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
        let ids = self
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        self.selected_file_ids = ids.iter().cloned().collect();
        if self.selected_file.is_none() {
            self.selected_file = ids.first().cloned();
        }
        self.selected_file_ids.len()
    }

    fn select_file_range_to(&mut self, id: String, add_to_existing: bool) {
        let files = self.selected_audio_files();
        let Some(target_index) = files.iter().position(|file| file.id == id) else {
            return;
        };
        let anchor = self
            .selected_file
            .as_deref()
            .and_then(|selected| files.iter().position(|file| file.id == selected))
            .unwrap_or(target_index);
        let start = anchor.min(target_index);
        let end = anchor.max(target_index);
        let range_ids = files[start..=end]
            .iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        drop(files);

        if !add_to_existing {
            self.selected_file_ids.clear();
        }
        self.selected_file_ids.extend(range_ids);
        self.selected_file = Some(id);
    }

    fn toggle_file_selection(&mut self, id: String) {
        if self.selected_file_ids.contains(&id) && self.selected_file_ids.len() > 1 {
            self.selected_file_ids.remove(&id);
        } else {
            self.selected_file_ids.insert(id.clone());
        }
        self.selected_file = Some(id);
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
}
