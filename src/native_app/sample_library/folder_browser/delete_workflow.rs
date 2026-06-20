use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::{
    FileDeleteTargetView, FolderBrowserState, FolderDeleteTargetView, path_helpers::path_id,
};

impl FolderBrowserState {
    pub(in crate::native_app) fn selected_delete_target(
        &self,
    ) -> Result<FolderDeleteTargetView, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before deleting a folder"));
        }
        if self.selection.selected_file_active() {
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

    pub(in crate::native_app) fn selected_file_delete_target(
        &self,
    ) -> Result<FileDeleteTargetView, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before deleting a file"));
        }
        if !self.selection.selected_file_active() {
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
    pub(in crate::native_app) fn delete_selected_folder(&mut self) -> Result<String, String> {
        let target = self.selected_delete_target()?;
        if !target.path.is_dir() {
            return Err(format!("Folder delete failed: {} is missing", target.name));
        }
        Err(String::from(
            "Trash workflow is not available in the default GUI yet; no folder was deleted",
        ))
    }

    #[cfg(test)]
    pub(in crate::native_app) fn delete_selected_files(&mut self) -> Result<String, String> {
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

    pub(in crate::native_app) fn discard_trashed_folder_path(&mut self, path: &Path) -> bool {
        let folder_id = path_id(path);
        let parent_id = path.parent().map(path_id);
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
        let changed = root_folder.remove_child_by_id(&folder_id);
        if !changed {
            return false;
        }
        self.tree.folders = vec![root_folder.clone()];
        let fallback_folder = parent_id
            .filter(|id| self.find_folder(id).is_some())
            .unwrap_or_else(|| {
                self.tree
                    .folders
                    .first()
                    .map(|folder| folder.id.clone())
                    .unwrap_or_default()
            });
        self.selection.discard_folder(&folder_id, fallback_folder);
        self.selection.clear_file_selection();
        self.tree.expanded_folders.retain(|id| id != &folder_id);
        self.bump_file_content_revision();
        true
    }

    #[cfg(test)]
    pub(in crate::native_app) fn discard_trashed_file_paths(&mut self, paths: &[PathBuf]) -> bool {
        self.discard_trashed_file_paths_matching_tags(paths, &HashMap::new())
    }

    pub(in crate::native_app) fn discard_trashed_file_paths_matching_tags(
        &mut self,
        paths: &[PathBuf],
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> bool {
        let target_ids = paths
            .iter()
            .map(|path| path_id(path))
            .collect::<HashSet<_>>();
        if target_ids.is_empty() {
            return false;
        }
        let focused_id = self.selection.selected_file_id().map(str::to_owned);
        let focused_removed = focused_id
            .as_deref()
            .is_some_and(|id| target_ids.contains(id));
        let before_visible_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
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
        let changed = root_folder.remove_files_by_ids(&target_ids);
        if !changed {
            return false;
        }
        self.tree.folders = vec![root_folder.clone()];
        self.bump_file_content_revision();
        let after_visible_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        let fallback_id = focused_removed
            .then(|| {
                fallback_after_deleted_focus(
                    focused_id.as_deref(),
                    &target_ids,
                    &before_visible_ids,
                    &after_visible_ids,
                )
            })
            .flatten();
        self.selection.discard_files(&target_ids);
        if let Some(fallback_id) = fallback_id {
            self.selection.set_focus_file_set(fallback_id);
        }
        self.reconcile_file_view_after_tagged_content_change(tags_by_file);
        true
    }
}

fn fallback_after_deleted_focus(
    focused_id: Option<&str>,
    removed_ids: &HashSet<String>,
    before_visible_ids: &[String],
    after_visible_ids: &[String],
) -> Option<String> {
    if after_visible_ids.is_empty() {
        return None;
    }
    let focused_id = focused_id.filter(|id| removed_ids.contains(*id))?;
    let before_index = before_visible_ids
        .iter()
        .position(|id| id == focused_id)
        .unwrap_or_default();
    let after_ids = after_visible_ids.iter().collect::<HashSet<_>>();
    before_visible_ids
        .iter()
        .skip(before_index.saturating_add(1))
        .find(|id| after_ids.contains(id))
        .or_else(|| {
            before_visible_ids
                .iter()
                .take(before_index)
                .rev()
                .find(|id| after_ids.contains(id))
        })
        .cloned()
}
