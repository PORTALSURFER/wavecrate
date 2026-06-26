use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use wavecrate::sample_sources::Rating;

use super::{
    FolderBrowserState, FolderEntry, FolderVerifyOutcome, FolderVerifyResult,
    file_refresh::RefreshedFileEntry,
    path_helpers::path_id,
    scanning::{file_entry_for_source_path, load_folder_at_path, upsert_file, upsert_folder},
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
        let source_database_root = source.database_root.clone();
        let Some(parent_folder) = root_folder.find_mut(&parent_id) else {
            return false;
        };
        upsert_file(
            &mut parent_folder.files,
            file_entry_for_source_path(&path.to_path_buf(), &source.root, &source_database_root),
        );
        self.tree.folders = vec![root_folder.clone()];
        self.bump_file_content_revision();
        self.refresh_missing_collection_state();
        true
    }

    pub(in crate::native_app) fn refresh_file_path_across_sources(&mut self, path: &Path) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .enumerate()
            .filter(|(_, source)| path.starts_with(&source.root))
            .max_by_key(|(_, source)| source.root.components().count())
            .map(|(index, _)| index)
        else {
            return false;
        };
        let source_id = self.source.sources[source_index].id.clone();
        let source_root = self.source.sources[source_index].root.clone();
        let source_database_root = self.source.sources[source_index].database_root.clone();
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        let Some(parent_folder) = root_folder.find_mut(&path_id(parent)) else {
            return false;
        };
        let changed = upsert_file(
            &mut parent_folder.files,
            file_entry_for_source_path(&path.to_path_buf(), &source_root, &source_database_root),
        );
        if !changed {
            return false;
        }
        if self.source.selected_source == source_id {
            self.tree.folders = vec![root_folder.clone()];
        }
        self.bump_file_content_revision();
        self.refresh_missing_collection_state();
        true
    }

    #[cfg(test)]
    pub(in crate::native_app) fn refresh_file_paths(&mut self, paths: &[PathBuf]) -> bool {
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == self.source.selected_source)
        else {
            return false;
        };
        let source_root = self.source.sources[source_index].root.clone();
        let source_database_root = self.source.sources[source_index].database_root.clone();
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
            return false;
        };

        let mut changed = false;
        for path in paths {
            let Some(parent) = path.parent() else {
                continue;
            };
            let Some(parent_folder) = root_folder.find_mut(&path_id(parent)) else {
                continue;
            };
            changed |= upsert_file(
                &mut parent_folder.files,
                file_entry_for_source_path(path, &source_root, &source_database_root),
            );
        }
        if !changed {
            return false;
        }

        self.tree.folders = vec![root_folder.clone()];
        self.bump_file_content_revision();
        self.refresh_missing_collection_state();
        true
    }

    pub(in crate::native_app) fn refresh_file_entries(
        &mut self,
        source_id: &str,
        entries: &[RefreshedFileEntry],
    ) -> bool {
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == source_id)
        else {
            return false;
        };
        let selected_source = self.source.selected_source == source_id;
        let (source_changed, root_id) = {
            let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
                return false;
            };
            let root_id = root_folder.id.clone();
            (upsert_refreshed_file_entries(root_folder, entries), root_id)
        };
        if !source_changed {
            return false;
        }

        if selected_source {
            let visible_root_found = self
                .tree
                .folders
                .iter_mut()
                .find(|folder| folder.id == root_id)
                .map(|root_folder| {
                    upsert_refreshed_file_entries(root_folder, entries);
                })
                .is_some();
            if !visible_root_found
                && let Some(root_folder) = self.source.sources[source_index].root_folder.clone()
            {
                self.tree.folders = vec![root_folder];
            }
        }
        self.bump_file_content_revision();
        self.refresh_missing_collection_state();
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
        let database_root = self.source.sources[source_index].database_root.clone();
        let mut changed = false;
        for relative_path in relative_paths {
            changed |= self.refresh_one_source_relative_path(
                source_index,
                &root,
                &database_root,
                relative_path,
            );
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
        self.refresh_missing_collection_state();
        true
    }

    fn refresh_one_source_relative_path(
        &mut self,
        source_index: usize,
        source_root: &Path,
        source_database_root: &Path,
        relative_path: &Path,
    ) -> bool {
        let absolute_path = source_root.join(relative_path);
        if absolute_path.is_dir() {
            return self.refresh_existing_folder_path(
                source_index,
                source_root,
                source_database_root,
                &absolute_path,
            );
        }
        if absolute_path.is_file() {
            return self.refresh_existing_file_path(
                source_index,
                source_database_root,
                &absolute_path,
            );
        }
        self.remove_missing_path_from_source(source_index, &absolute_path)
    }

    fn refresh_existing_file_path(
        &mut self,
        source_index: usize,
        source_database_root: &Path,
        path: &Path,
    ) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let parent_id = path_id(parent);
        let source_root = self.source.sources[source_index].root.clone();
        let changed = {
            let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
                return false;
            };
            if root_folder.find(&parent_id).is_none() {
                let source_root = self.source.sources[source_index].root.clone();
                return self.refresh_existing_folder_path(
                    source_index,
                    &source_root,
                    source_database_root,
                    parent,
                );
            }
            let Some(parent_folder) = root_folder.find_mut(&parent_id) else {
                return false;
            };
            upsert_file(
                &mut parent_folder.files,
                file_entry_for_source_path(&path.to_path_buf(), &source_root, source_database_root),
            )
        };
        if changed {
            self.source.sources[source_index]
                .missing_collection_snapshot
                .remove_path(path);
        }
        changed
    }

    fn refresh_existing_folder_path(
        &mut self,
        source_index: usize,
        source_root: &Path,
        source_database_root: &Path,
        path: &Path,
    ) -> bool {
        let Some(folder) = load_folder_at_path(path, source_root, source_database_root) else {
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
            self.source.sources[source_index]
                .missing_collection_snapshot
                .remove_prefix(path);
            return true;
        }
        let Some(parent) = path.parent() else {
            return false;
        };
        let Some(parent_folder) = root_folder.find_mut(&path_id(parent)) else {
            return false;
        };
        let changed = upsert_folder(&mut parent_folder.children, folder);
        if changed {
            self.source.sources[source_index]
                .missing_collection_snapshot
                .remove_prefix(path);
        }
        changed
    }

    fn remove_missing_path_from_source(&mut self, source_index: usize, path: &Path) -> bool {
        let path_id = path_id(path);
        let removed_folder;
        let removed_file;
        {
            let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
                return false;
            };
            removed_folder = root_folder.take_child_by_id(&path_id);
            removed_file = root_folder.take_file_by_id(&path_id);
        }
        let changed = removed_folder.is_some() || removed_file.is_some();
        if changed {
            let snapshot = &mut self.source.sources[source_index].missing_collection_snapshot;
            if let Some(folder) = &removed_folder {
                snapshot.add_missing_files_from_folder(folder);
            }
            if let Some(file) = removed_file {
                snapshot.add_missing_file(file);
            }
        }
        changed
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
                let visible_ids = self
                    .selected_audio_files()
                    .into_iter()
                    .map(|file| file.id.clone())
                    .collect::<HashSet<_>>();
                self.selection.retain_visible_files(&visible_ids);
            }
            self.bump_file_content_revision();
            self.refresh_missing_collection_state();
        }
    }

    pub(in crate::native_app) fn set_file_rating_state(
        &mut self,
        path: &Path,
        rating: Rating,
        locked: bool,
    ) -> bool {
        let Some(source_index) = self.source_index_for_path(path) else {
            return false;
        };
        let file_id = path_id(path);
        let changed = self.source.sources[source_index]
            .root_folder
            .as_mut()
            .is_some_and(|root| root.set_file_rating(&file_id, rating, locked));
        if !changed {
            return false;
        }
        self.update_visible_tree_file_rating(&file_id, source_index, rating, locked);
        let curated_at = super::curation::now_epoch_seconds();
        let _ = self.source.sources[source_index]
            .root_folder
            .as_mut()
            .is_some_and(|root| root.set_file_last_curated_at(&file_id, curated_at));
        self.update_visible_tree_file_last_curated_at(&file_id, source_index, curated_at);
        self.bump_file_content_revision();
        true
    }

    pub(in crate::native_app) fn set_file_last_played_at(
        &mut self,
        path: &Path,
        last_played_at: i64,
    ) -> bool {
        let Some(source_index) = self.source_index_for_path(path) else {
            return false;
        };
        let file_id = path_id(path);
        let changed = self.source.sources[source_index]
            .root_folder
            .as_mut()
            .is_some_and(|root| root.set_file_last_played_at(&file_id, last_played_at));
        if !changed {
            return false;
        }
        self.update_visible_tree_file_last_played_at(&file_id, source_index, last_played_at);
        self.bump_file_content_revision();
        true
    }

    pub(in crate::native_app) fn set_file_last_curated_at(
        &mut self,
        path: &Path,
        last_curated_at: i64,
    ) -> bool {
        let Some(source_index) = self.source_index_for_path(path) else {
            return false;
        };
        let file_id = path_id(path);
        let changed = self.source.sources[source_index]
            .root_folder
            .as_mut()
            .is_some_and(|root| root.set_file_last_curated_at(&file_id, last_curated_at));
        if !changed {
            return false;
        }
        self.update_visible_tree_file_last_curated_at(&file_id, source_index, last_curated_at);
        self.bump_file_content_revision();
        true
    }

    pub(in crate::native_app) fn set_file_ids_last_curated_at(
        &mut self,
        file_ids: &[String],
        last_curated_at: i64,
    ) -> bool {
        if file_ids.is_empty() {
            return false;
        }
        let target_ids = file_ids.iter().cloned().collect::<HashSet<_>>();
        let mut changed = false;
        let mut visible_changed = false;
        for source_index in 0..self.source.sources.len() {
            let source_changed = self.source.sources[source_index]
                .root_folder
                .as_mut()
                .is_some_and(|root| root.set_files_last_curated_at(&target_ids, last_curated_at));
            changed |= source_changed;
            visible_changed |= source_changed && self.source_is_visible(source_index);
        }
        if !changed {
            return false;
        }
        if visible_changed {
            for root in &mut self.tree.folders {
                root.set_files_last_curated_at(&target_ids, last_curated_at);
            }
        }
        self.bump_file_content_revision();
        true
    }

    fn source_index_for_path(&self, path: &Path) -> Option<usize> {
        self.source
            .sources
            .iter()
            .enumerate()
            .filter(|(_, source)| path.starts_with(&source.root))
            .max_by_key(|(_, source)| source.root.components().count())
            .map(|(index, _)| index)
    }

    fn source_is_visible(&self, source_index: usize) -> bool {
        self.source
            .sources
            .get(source_index)
            .is_some_and(|source| source.id == self.source.selected_source)
    }

    fn update_visible_tree_file_rating(
        &mut self,
        file_id: &str,
        source_index: usize,
        rating: Rating,
        locked: bool,
    ) {
        if !self.source_is_visible(source_index) {
            return;
        }
        for root in &mut self.tree.folders {
            if root.set_file_rating(file_id, rating, locked) {
                break;
            }
        }
    }

    fn update_visible_tree_file_last_played_at(
        &mut self,
        file_id: &str,
        source_index: usize,
        last_played_at: i64,
    ) {
        if !self.source_is_visible(source_index) {
            return;
        }
        for root in &mut self.tree.folders {
            if root.set_file_last_played_at(file_id, last_played_at) {
                break;
            }
        }
    }

    fn update_visible_tree_file_last_curated_at(
        &mut self,
        file_id: &str,
        source_index: usize,
        last_curated_at: i64,
    ) {
        if !self.source_is_visible(source_index) {
            return;
        }
        for root in &mut self.tree.folders {
            if root.set_file_last_curated_at(file_id, last_curated_at) {
                break;
            }
        }
    }
}

fn upsert_refreshed_file_entries(
    root_folder: &mut FolderEntry,
    entries: &[RefreshedFileEntry],
) -> bool {
    let mut changed = false;
    for entry in entries {
        let path = entry.path();
        let Some(parent) = path.parent() else {
            continue;
        };
        let Some(parent_folder) = root_folder.find_mut(&path_id(parent)) else {
            continue;
        };
        changed |= upsert_file(&mut parent_folder.files, entry.file.clone());
    }
    changed
}
