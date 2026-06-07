use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{
    FileDeleteTargetView, FolderBrowserState, FolderDeleteTargetView, path_helpers::path_id,
};

impl FolderBrowserState {
    pub(in crate::gui_app) fn selected_delete_target(
        &self,
    ) -> Result<FolderDeleteTargetView, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before deleting a folder"));
        }
        if self.selected_file.is_some() {
            return Err(String::from("Select a folder to delete"));
        }
        let Some(folder) = self.selected_folder() else {
            return Err(String::from("Select a folder to delete"));
        };
        if self.selected_folder_is_source_root() {
            return Err(String::from("Root folder cannot be deleted"));
        }
        Ok(FolderDeleteTargetView {
            path: PathBuf::from(&folder.id),
            name: folder.name.clone(),
        })
    }

    pub(in crate::gui_app) fn selected_file_delete_target(
        &self,
    ) -> Result<FileDeleteTargetView, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before deleting a file"));
        }
        if self.selected_file.is_none() {
            return Err(String::from("Select a file to delete"));
        }
        let paths = self.selected_file_paths();
        if paths.is_empty() {
            return Err(String::from("Select a file to delete"));
        }
        let names = paths
            .iter()
            .map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            })
            .collect();
        Ok(FileDeleteTargetView { paths, names })
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn delete_selected_folder(&mut self) -> Result<String, String> {
        let target = self.selected_delete_target()?;
        if !target.path.is_dir() {
            return Err(format!("Folder delete failed: {} is missing", target.name));
        }
        Err(String::from(
            "Trash workflow is not available in the default GUI yet; no folder was deleted",
        ))
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn delete_selected_files(&mut self) -> Result<String, String> {
        let target = self.selected_file_delete_target()?;
        for path in &target.paths {
            if !path.is_file() {
                return Err(format!(
                    "File delete failed: {} is missing",
                    path.file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string())
                ));
            }
        }
        Err(String::from(
            "Trash workflow is not available in the default GUI yet; no files were deleted",
        ))
    }

    pub(in crate::gui_app) fn discard_trashed_folder_path(&mut self, path: &Path) -> bool {
        let folder_id = path_id(path);
        let parent_id = path.parent().map(path_id);
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
        let changed = root_folder.remove_child_by_id(&folder_id);
        if !changed {
            return false;
        }
        self.folders = vec![root_folder.clone()];
        if self.selected_folder == folder_id {
            self.selected_folder = parent_id
                .filter(|id| self.find_folder(id).is_some())
                .unwrap_or_else(|| {
                    self.folders
                        .first()
                        .map(|folder| folder.id.clone())
                        .unwrap_or_default()
                });
        }
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.selected_file_ids_explicit = false;
        self.expanded_folders.retain(|id| id != &folder_id);
        self.bump_file_content_revision();
        true
    }

    pub(in crate::gui_app) fn discard_trashed_file_paths(&mut self, paths: &[PathBuf]) -> bool {
        let target_ids = paths
            .iter()
            .map(|path| path_id(path))
            .collect::<HashSet<_>>();
        if target_ids.is_empty() {
            return false;
        }
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
        let changed = root_folder.remove_files_by_ids(&target_ids);
        if !changed {
            return false;
        }
        self.folders = vec![root_folder.clone()];
        if self
            .selected_file
            .as_ref()
            .is_some_and(|id| target_ids.contains(id))
        {
            self.selected_file = None;
        }
        self.selected_file_ids.retain(|id| !target_ids.contains(id));
        self.bump_file_content_revision();
        true
    }
}
