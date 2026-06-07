use std::path::Path;

use super::{
    FolderBrowserState, FolderEntry, FolderRenameKind,
    path_helpers::{path_id, rewrite_path_id},
    scanning::upsert_folder,
};

impl FolderBrowserState {
    pub(super) fn rewrite_renamed_folder_paths(&mut self, old_path: &Path, new_path: &Path) {
        let old_id = path_id(old_path);
        let new_id = path_id(new_path);
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return;
        };
        if let Some(root_folder) = &mut source.root_folder {
            root_folder.rewrite_path_prefix(old_path, new_path);
            self.folders = vec![root_folder.clone()];
        }
        self.selected_folder = rewrite_path_id(&self.selected_folder, old_path, new_path);
        if self.selected_folder == old_id {
            self.selected_folder = new_id;
        }
        self.selected_file = self
            .selected_file
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selected_file_ids = self
            .selected_file_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.expanded_folders = self
            .expanded_folders
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.bump_file_content_revision();
    }

    pub(super) fn rewrite_renamed_file_path(&mut self, old_path: &Path, new_path: &Path) {
        let Some(source) = self
            .sources
            .iter_mut()
            .find(|source| source.id == self.selected_source)
        else {
            return;
        };
        if let Some(root_folder) = &mut source.root_folder {
            root_folder.rewrite_file_path(old_path, new_path);
            self.folders = vec![root_folder.clone()];
        }
        let new_id = path_id(new_path);
        self.selected_file = Some(new_id);
        self.selected_file_ids.clear();
        self.selected_file_ids.insert(path_id(new_path));
        self.selected_file_ids_explicit = false;
        self.bump_file_content_revision();
    }

    pub(super) fn discard_pending_created_folder(&mut self) {
        let Some(edit) = self.rename_edit.take() else {
            return;
        };
        if let FolderRenameKind::Create { parent_id } = edit.kind {
            self.remove_pending_created_folder(&edit.folder_id, &parent_id);
        }
    }

    pub(super) fn remove_pending_created_folder(&mut self, folder_id: &str, parent_id: &str) {
        self.remove_folder_by_id(folder_id);
        if self.selected_folder == folder_id {
            self.selected_folder = if self.find_folder(parent_id).is_some() {
                parent_id.to_string()
            } else {
                self.folders
                    .first()
                    .map(|folder| folder.id.clone())
                    .unwrap_or_default()
            };
        }
        self.expanded_folders.remove(folder_id);
    }

    fn remove_folder_by_id(&mut self, folder_id: &str) -> bool {
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
        let changed = root_folder.remove_child_by_id(folder_id);
        if changed {
            self.folders = vec![root_folder.clone()];
            self.bump_file_content_revision();
        }
        changed
    }

    pub(super) fn upsert_child_folder(&mut self, parent_id: &str, folder: FolderEntry) -> bool {
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
        let Some(parent) = root_folder.find_mut(parent_id) else {
            return false;
        };
        let changed = upsert_folder(&mut parent.children, folder);
        if changed {
            self.folders = vec![root_folder.clone()];
            self.bump_file_content_revision();
        }
        changed
    }
}
