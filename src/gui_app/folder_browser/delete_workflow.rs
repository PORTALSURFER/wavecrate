use std::path::PathBuf;

use super::{FileDeleteTargetView, FolderBrowserState, FolderDeleteTargetView};

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

    pub(in crate::gui_app) fn delete_selected_folder(&mut self) -> Result<String, String> {
        let target = self.selected_delete_target()?;
        if !target.path.is_dir() {
            return Err(format!("Folder delete failed: {} is missing", target.name));
        }
        Err(String::from(
            "Trash workflow is not available in the default GUI yet; no folder was deleted",
        ))
    }

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
}
