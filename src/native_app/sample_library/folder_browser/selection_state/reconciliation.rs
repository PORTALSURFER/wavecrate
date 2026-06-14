use super::{BrowserSelectionSnapshot, BrowserSelectionState};
use crate::native_app::sample_library::folder_browser::path_helpers::rewrite_path_id;
use std::collections::HashSet;
use std::path::Path;

impl BrowserSelectionState {
    pub(in crate::native_app::sample_library::folder_browser) fn rewrite_folder_prefix(
        &mut self,
        old_path: &Path,
        new_path: &Path,
    ) {
        self.selected_folder = rewrite_path_id(&self.selected_folder, old_path, new_path);
        self.selected_folder_ids = self
            .selected_folder_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.folder_selection_anchor = self
            .folder_selection_anchor
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selected_file = self
            .selected_file
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selected_file_ids = self
            .selected_file_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
    }

    pub(in crate::native_app::sample_library::folder_browser) fn set_renamed_file(
        &mut self,
        new_id: String,
    ) {
        self.set_focus_file_set(new_id);
    }

    pub(in crate::native_app::sample_library::folder_browser) fn restore_after_moved_files(
        &mut self,
        snapshot: BrowserSelectionSnapshot,
        moved_old_ids: &HashSet<String>,
        visible_ids: &[String],
    ) {
        let visible_id_set = visible_ids.iter().cloned().collect::<HashSet<_>>();
        self.selected_folder = snapshot.selected_folder;
        self.selected_file_ids = snapshot
            .selected_file_ids
            .into_iter()
            .filter(|id| !moved_old_ids.contains(id) && visible_id_set.contains(id))
            .collect();
        self.selected_file_ids_explicit = snapshot.selected_file_ids_explicit;
        self.selected_file = snapshot
            .selected_file
            .filter(|id| !moved_old_ids.contains(id) && visible_id_set.contains(id))
            .or_else(|| {
                visible_ids
                    .iter()
                    .find(|id| self.selected_file_ids.contains(*id))
                    .cloned()
            });

        if self.selected_file.is_none()
            && self.selected_file_ids.is_empty()
            && let Some(first_visible) = visible_ids.first().cloned()
        {
            self.set_focus_file_set(first_visible);
        }
    }

    pub(in crate::native_app::sample_library::folder_browser) fn select_moved_files(
        &mut self,
        target_folder_id: String,
        moved_file_ids: &[(String, String)],
    ) {
        let selected_file_was_moved = self
            .selected_file
            .as_ref()
            .is_some_and(|id| moved_file_ids.iter().any(|(old_id, _)| old_id == id));
        self.selected_file = if selected_file_was_moved {
            self.selected_file.take().map(|id| {
                moved_file_ids
                    .iter()
                    .find(|(old_id, _)| old_id == &id)
                    .map(|(_, new_id)| new_id.clone())
                    .unwrap_or(id)
            })
        } else {
            moved_file_ids.first().map(|(_, new_id)| new_id.clone())
        };
        self.selected_file_ids = if self
            .selected_file_ids
            .iter()
            .any(|id| moved_file_ids.iter().any(|(old_id, _)| old_id == id))
        {
            self.selected_file_ids
                .iter()
                .map(|id| {
                    moved_file_ids
                        .iter()
                        .find(|(old_id, _)| old_id == id)
                        .map(|(_, new_id)| new_id.clone())
                        .unwrap_or_else(|| id.clone())
                })
                .collect()
        } else {
            moved_file_ids
                .iter()
                .map(|(_, new_id)| new_id.clone())
                .collect()
        };
        self.selected_folder = target_folder_id;
        self.selected_file_ids_explicit = self.selected_file_ids.len() > 1;
    }

    pub(in crate::native_app::sample_library::folder_browser) fn discard_files(
        &mut self,
        removed_ids: &HashSet<String>,
    ) {
        if self
            .selected_file
            .as_ref()
            .is_some_and(|id| removed_ids.contains(id))
        {
            self.selected_file = None;
        }
        self.selected_file_ids
            .retain(|id| !removed_ids.contains(id));
        if self.selected_file.is_none() && self.selected_file_ids.is_empty() {
            self.selected_file_ids_explicit = false;
        }
    }
}
