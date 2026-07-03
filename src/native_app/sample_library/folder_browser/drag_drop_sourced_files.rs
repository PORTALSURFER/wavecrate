use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use super::{
    FileMoveItem, FolderBrowserState,
    path_helpers::path_id,
    placeholder_folder,
    scanning::{file_entry_for_source_path, upsert_file},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourcedMovedFile {
    pub(super) source_root: PathBuf,
    pub(super) source_database_root: PathBuf,
    pub(super) old_path: PathBuf,
    pub(super) new_path: PathBuf,
    pub(super) copy_only: bool,
}

impl FolderBrowserState {
    pub(super) fn relocate_sourced_moved_files(
        &mut self,
        moves: &[SourcedMovedFile],
        target_source_root: &Path,
        target_parent: &Path,
    ) -> Result<(), String> {
        let Some(target_source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.root == target_source_root)
        else {
            return Err(String::from(
                "File move failed: target source is unavailable",
            ));
        };
        let target_source_root = self.source.sources[target_source_index].root.clone();
        let target_source_database_root = self.source.sources[target_source_index]
            .database_root
            .clone();
        self.materialize_sourced_move_target_tree(target_source_index);
        let target_parent_id = path_id(target_parent);
        let old_ids = moves
            .iter()
            .filter(|move_item| !move_item.copy_only)
            .map(|move_item| path_id(&move_item.old_path))
            .collect::<HashSet<_>>();
        let previous_files = self.previous_files_for_moves(&old_ids);

        for source_index in 0..self.source.sources.len() {
            let source_root = self.source.sources[source_index].root.clone();
            let source_old_ids = moves
                .iter()
                .filter(|move_item| !move_item.copy_only)
                .filter(|move_item| move_item.source_root == source_root)
                .map(|move_item| path_id(&move_item.old_path))
                .collect::<HashSet<_>>();
            if source_old_ids.is_empty() {
                continue;
            }
            if let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() {
                root_folder.remove_files_by_ids(&source_old_ids);
            }
        }

        let Some(root_folder) = self.source.sources[target_source_index]
            .root_folder
            .as_mut()
        else {
            return Err(String::from("File move failed: target tree is unavailable"));
        };
        let Some(target_folder) = root_folder.find_mut(&target_parent_id) else {
            return Err(String::from(
                "File move failed: target folder is unavailable",
            ));
        };
        for move_item in moves {
            let mut moved_file = file_entry_for_source_path(
                &move_item.new_path,
                &target_source_root,
                &target_source_database_root,
            );
            let old_id = path_id(&move_item.old_path);
            if let Some(previous) = previous_files.get(&old_id)
                && moved_file.rating.is_neutral()
                && !previous.rating.is_neutral()
            {
                moved_file.rating = previous.rating;
                moved_file.rating_locked = previous.rating_locked;
                moved_file.collection = previous.collection;
                moved_file.collections = previous.collections.clone();
            }
            upsert_file(&mut target_folder.files, moved_file);
        }

        let mut missing_changed = false;
        for source in &mut self.source.sources {
            for move_item in moves {
                if !move_item.copy_only && move_item.old_path.starts_with(&source.root) {
                    missing_changed |= source
                        .missing_collection_snapshot
                        .remove_path(&move_item.old_path);
                }
                if move_item.new_path.starts_with(&source.root) {
                    missing_changed |= source
                        .missing_collection_snapshot
                        .remove_path(&move_item.new_path);
                }
            }
        }
        if let Some(selected_root_folder) = self
            .source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)
            .and_then(|source| source.root_folder.clone())
        {
            self.tree.folders = vec![selected_root_folder];
        }
        let moved_file_ids = moves
            .iter()
            .map(|move_item| (path_id(&move_item.old_path), path_id(&move_item.new_path)))
            .collect::<Vec<_>>();
        for move_item in moves {
            if !move_item.copy_only {
                self.rewrite_similarity_path_prefix(&move_item.old_path, &move_item.new_path);
            }
        }
        self.selection
            .select_moved_files(target_parent_id.clone(), &moved_file_ids);
        self.tree.expanded_folders.insert(target_parent_id);
        self.bump_file_content_revision();
        if missing_changed {
            self.refresh_missing_collection_state();
        }
        Ok(())
    }

    fn materialize_sourced_move_target_tree(&mut self, target_source_index: usize) {
        if self.source.sources[target_source_index]
            .root_folder
            .is_some()
        {
            return;
        }
        let selected_target =
            self.source.selected_source == self.source.sources[target_source_index].id;
        if selected_target && let Some(root_folder) = self.tree.folders.first().cloned() {
            self.source.sources[target_source_index].root_folder = Some(root_folder);
            return;
        }
        let root = self.source.sources[target_source_index].root.clone();
        self.source.sources[target_source_index].root_folder = Some(placeholder_folder(&root));
    }

    fn previous_files_for_moves(
        &self,
        old_ids: &HashSet<String>,
    ) -> HashMap<String, super::FileEntry> {
        let mut previous_files = HashMap::new();
        for source in &self.source.sources {
            let Some(root_folder) = &source.root_folder else {
                continue;
            };
            for old_id in old_ids {
                if let Some(file) = root_folder.find_file(old_id) {
                    previous_files.insert(old_id.clone(), file.clone());
                }
            }
        }
        previous_files
    }
}

pub(super) fn sourced_moved_files_from_items(
    file_moves: &[FileMoveItem],
    moved_paths: &[(PathBuf, PathBuf)],
) -> Vec<SourcedMovedFile> {
    let source_roots = file_moves
        .iter()
        .map(|item| {
            (
                PathBuf::from(&item.file_id),
                (
                    item.source_root.clone(),
                    item.source_database_root.clone(),
                    item.copy_only,
                ),
            )
        })
        .collect::<HashMap<_, _>>();
    moved_paths
        .iter()
        .filter_map(|(old_path, new_path)| {
            source_roots.get(old_path).cloned().map(
                |(source_root, source_database_root, copy_only)| SourcedMovedFile {
                    source_root,
                    source_database_root,
                    old_path: old_path.clone(),
                    new_path: new_path.clone(),
                    copy_only,
                },
            )
        })
        .collect()
}

pub(super) fn sourced_file_move_metadata_from_sourced_moves(
    moves: &[SourcedMovedFile],
) -> Vec<wavecrate::sample_sources::SourcedFileMoveMetadata> {
    moves
        .iter()
        .map(
            |move_item| wavecrate::sample_sources::SourcedFileMoveMetadata {
                source_root: move_item.source_root.clone(),
                source_database_root: move_item.source_database_root.clone(),
                old_path: move_item.old_path.clone(),
                new_path: move_item.new_path.clone(),
                remove_source: !move_item.copy_only,
            },
        )
        .collect()
}
