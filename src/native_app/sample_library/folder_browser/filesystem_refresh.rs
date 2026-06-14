use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use wavecrate::sample_sources::Rating;

use super::{
    FolderBrowserState, FolderVerifyOutcome, FolderVerifyResult,
    path_helpers::path_id,
    scanning::{
        file_entry, file_entry_for_source_path, load_folder_at_path, upsert_file, upsert_folder,
    },
};

impl FolderBrowserState {
    pub(in crate::native_app) fn refresh_file_path(&mut self, path: &Path) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let parent_id = path_id(parent);
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
        let Some(parent_folder) = root_folder.find_mut(&parent_id) else {
            return false;
        };
        upsert_file(
            &mut parent_folder.files,
            file_entry_for_source_path(&path.to_path_buf(), &source.root),
        );
        self.tree.folders = vec![root_folder.clone()];
        self.bump_file_content_revision();
        true
    }

    pub(in crate::native_app) fn refresh_filesystem_paths(
        &mut self,
        source_id: &str,
        relative_paths: &[PathBuf],
    ) -> bool {
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == source_id)
        else {
            return false;
        };
        let root = self.source.sources[source_index].root.clone();
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
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == result.source_id)
        else {
            return false;
        };
        let snapshot = match result.outcome {
            FolderVerifyOutcome::Unchanged => return false,
            FolderVerifyOutcome::Missing => {
                let changed =
                    self.remove_missing_path_from_source(source_index, &result.folder_path);
                if changed {
                    self.after_source_tree_changed(&result.source_id);
                }
                return changed;
            }
            FolderVerifyOutcome::Changed(snapshot) => snapshot,
        };
        let folder_id = path_id(&result.folder_path);
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        let Some(folder) = root_folder.find_mut(&folder_id) else {
            return false;
        };
        if !folder.replace_direct_entries(snapshot.child_paths, snapshot.files) {
            return false;
        }
        if self.source.selected_source == result.source_id {
            self.tree.folders = vec![root_folder.clone()];
            if self.selection.selected_folder == folder_id {
                let visible_ids = self
                    .selected_audio_files()
                    .into_iter()
                    .map(|file| file.id.clone())
                    .collect::<HashSet<_>>();
                self.selection.retain_visible_files(&visible_ids);
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
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        if root_folder.find(&parent_id).is_none() {
            let source_root = self.source.sources[source_index].root.clone();
            return self.refresh_existing_folder_path(source_index, &source_root, parent);
        }
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
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
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
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
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        let removed_folder = root_folder.remove_child_by_id(&path_id);
        let removed_file = root_folder.remove_file_by_id(&path_id);
        removed_folder || removed_file
    }

    fn after_source_tree_changed(&mut self, source_id: &str) {
        if let Some(root_folder) = self
            .source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .and_then(|source| source.root_folder.clone())
        {
            if self.source.selected_source == source_id {
                if root_folder.find(&self.selection.selected_folder).is_none() {
                    self.selection
                        .select_folder_after_tree_changed(root_folder.id.clone());
                }
                self.tree.folders = vec![root_folder];
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
        for source in &mut self.source.sources {
            let Some(root_folder) = &mut source.root_folder else {
                continue;
            };
            changed |= root_folder.set_file_rating(&file_id, rating, locked);
            if source.id == self.source.selected_source {
                self.tree.folders = vec![root_folder.clone()];
            }
        }
        if changed {
            self.bump_file_content_revision();
        }
        changed
    }
}
