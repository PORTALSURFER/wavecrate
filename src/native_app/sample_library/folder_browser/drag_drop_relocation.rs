use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use super::{
    FolderBrowserState,
    path_helpers::{path_id, rewrite_path_id},
    scanning::{file_entry, upsert_file, upsert_folder},
};

impl FolderBrowserState {
    pub(super) fn relocate_moved_folder(
        &mut self,
        old_path: &Path,
        new_path: &Path,
        target_parent: &Path,
    ) -> Result<(), String> {
        let old_id = path_id(old_path);
        let target_parent_id = path_id(target_parent);
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
        else {
            return Err(String::from(
                "Folder move failed: selected source is unavailable",
            ));
        };
        let Some(root_folder) = &mut source.root_folder else {
            return Err(String::from(
                "Folder move failed: source tree is unavailable",
            ));
        };
        let Some(mut moved_folder) = root_folder.take_child_by_id(&old_id) else {
            return Err(String::from(
                "Folder move failed: source folder is unavailable",
            ));
        };
        moved_folder.rewrite_path_prefix(old_path, new_path);
        let Some(target_folder) = root_folder.find_mut(&target_parent_id) else {
            return Err(String::from(
                "Folder move failed: target folder is unavailable",
            ));
        };
        upsert_folder(&mut target_folder.children, moved_folder);
        self.tree.folders = vec![root_folder.clone()];

        self.selection.selected_folder =
            rewrite_path_id(&self.selection.selected_folder, old_path, new_path);
        self.selection.selected_file = self
            .selection
            .selected_file
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selection.selected_file_ids = self
            .selection
            .selected_file_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.tree.expanded_folders = self
            .tree
            .expanded_folders
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.tree.expanded_folders.insert(target_parent_id);
        self.bump_file_content_revision();
        Ok(())
    }

    pub(super) fn relocate_moved_files(
        &mut self,
        moves: &[(PathBuf, PathBuf)],
        target_parent: &Path,
    ) -> Result<(), String> {
        let old_ids = moves
            .iter()
            .map(|(old_path, _)| path_id(old_path))
            .collect::<HashSet<_>>();
        let target_parent_id = path_id(target_parent);
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
        else {
            return Err(String::from(
                "File move failed: selected source is unavailable",
            ));
        };
        let Some(root_folder) = &mut source.root_folder else {
            return Err(String::from("File move failed: source tree is unavailable"));
        };
        root_folder.remove_files_by_ids(&old_ids);
        let Some(target_folder) = root_folder.find_mut(&target_parent_id) else {
            return Err(String::from(
                "File move failed: target folder is unavailable",
            ));
        };
        for (_, new_path) in moves {
            upsert_file(&mut target_folder.files, file_entry(new_path));
        }
        self.tree.folders = vec![root_folder.clone()];
        let moved_ids = moves
            .iter()
            .map(|(_, new_path)| path_id(new_path))
            .collect::<Vec<_>>();
        let rewrite_file_id = |id: &str| {
            moves
                .iter()
                .find(|(old_path, _)| path_id(old_path) == id)
                .map(|(_, new_path)| path_id(new_path))
                .unwrap_or_else(|| id.to_string())
        };
        let selected_file_was_moved = self
            .selection
            .selected_file
            .as_ref()
            .is_some_and(|id| old_ids.contains(id));
        self.selection.selected_file = if selected_file_was_moved {
            self.selection
                .selected_file
                .take()
                .map(|id| rewrite_file_id(&id))
        } else {
            moved_ids.first().cloned()
        };
        self.selection.selected_file_ids = if self
            .selection
            .selected_file_ids
            .iter()
            .any(|id| old_ids.contains(id))
        {
            self.selection
                .selected_file_ids
                .iter()
                .map(|id| rewrite_file_id(id))
                .collect()
        } else {
            moved_ids.iter().cloned().collect()
        };
        self.selection.selected_folder = target_parent_id.clone();
        self.reset_file_view();
        self.tree.expanded_folders.insert(target_parent_id);
        self.bump_file_content_revision();
        Ok(())
    }
}
