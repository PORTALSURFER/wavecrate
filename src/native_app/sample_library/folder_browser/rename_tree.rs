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
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
        else {
            return;
        };
        if let Some(root_folder) = &mut source.root_folder {
            root_folder.rewrite_path_prefix(old_path, new_path);
            self.tree.folders = vec![root_folder.clone()];
        }
        self.selection.rewrite_folder_prefix(old_path, new_path);
        self.rewrite_similarity_path_prefix(old_path, new_path);
        if self.selection.selected_folder_id() == old_id {
            self.selection.set_folder_focus(new_id);
        }
        self.tree.expanded_folders = self
            .tree
            .expanded_folders
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.bump_file_content_revision();
    }

    pub(super) fn rewrite_renamed_file_path(&mut self, old_path: &Path, new_path: &Path) {
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
        else {
            return;
        };
        if let Some(root_folder) = &mut source.root_folder {
            root_folder.rewrite_file_path(old_path, new_path);
            self.tree.folders = vec![root_folder.clone()];
        }
        self.selection.set_renamed_file(path_id(new_path));
        self.rewrite_similarity_path_prefix(old_path, new_path);
        self.bump_file_content_revision();
    }

    pub(super) fn discard_pending_created_folder(&mut self) {
        let Some(edit) = self.rename.folder.take() else {
            return;
        };
        if let FolderRenameKind::Create { parent_id } = edit.kind {
            self.remove_pending_created_folder(&edit.folder_id, &parent_id);
        }
    }

    pub(super) fn remove_pending_created_folder(&mut self, folder_id: &str, parent_id: &str) {
        self.remove_folder_by_id(folder_id);
        if self.selection.selected_folder_id() == folder_id {
            let selected_folder = if self.find_folder(parent_id).is_some() {
                parent_id.to_string()
            } else {
                self.tree
                    .folders
                    .first()
                    .map(|folder| folder.id.clone())
                    .unwrap_or_default()
            };
            self.selection.set_folder_focus(selected_folder);
        }
        self.tree.expanded_folders.remove(folder_id);
    }

    fn remove_folder_by_id(&mut self, folder_id: &str) -> bool {
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
        else {
            return false;
        };
        let Some(root_folder) = &mut source.root_folder else {
            return false;
        };
        let changed = root_folder.remove_child_by_id(folder_id);
        if changed {
            self.tree.folders = vec![root_folder.clone()];
            self.bump_file_content_revision();
        }
        changed
    }

    pub(super) fn upsert_child_folder(&mut self, parent_id: &str, folder: FolderEntry) -> bool {
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
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
            self.tree.folders = vec![root_folder.clone()];
            self.bump_file_content_revision();
        }
        changed
    }
}
