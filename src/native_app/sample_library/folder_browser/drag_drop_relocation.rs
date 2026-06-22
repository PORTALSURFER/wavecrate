use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use super::{
    FolderBrowserState,
    path_helpers::{path_id, rewrite_path_id},
    scanning::{file_entry_for_source_path, upsert_file, upsert_folder},
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

        self.selection.rewrite_folder_prefix(old_path, new_path);
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
        let Some(source_root) = self
            .source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)
            .map(|source| source.root.clone())
        else {
            return Err(String::from(
                "File move failed: selected source is unavailable",
            ));
        };
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
        let previous_files = moves
            .iter()
            .filter_map(|(old_path, _)| {
                let old_id = path_id(old_path);
                root_folder
                    .find_file(&old_id)
                    .cloned()
                    .map(|file| (old_id, file))
            })
            .collect::<HashMap<_, _>>();
        root_folder.remove_files_by_ids(&old_ids);
        let Some(target_folder) = root_folder.find_mut(&target_parent_id) else {
            return Err(String::from(
                "File move failed: target folder is unavailable",
            ));
        };
        for (old_path, new_path) in moves {
            let mut moved_file = file_entry_for_source_path(new_path, &source_root);
            let old_id = path_id(old_path);
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
        self.tree.folders = vec![root_folder.clone()];
        let moved_file_ids = moves
            .iter()
            .map(|(old_path, new_path)| (path_id(old_path), path_id(new_path)))
            .collect::<Vec<_>>();
        self.selection
            .select_moved_files(target_parent_id.clone(), &moved_file_ids);
        self.tree.expanded_folders.insert(target_parent_id);
        self.bump_file_content_revision();
        Ok(())
    }
}

pub(super) fn persist_moved_folders_metadata(
    source_root: &Path,
    moves: &[(PathBuf, PathBuf)],
) -> Result<(), String> {
    let errors = moves
        .iter()
        .filter_map(|(old_path, new_path)| {
            persist_moved_folder_metadata(source_root, old_path, new_path)
                .err()
                .map(|error| format!("{}: {error}", old_path.display()))
        })
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn persist_moved_folder_metadata(
    source_root: &Path,
    old_path: &Path,
    new_path: &Path,
) -> Result<(), String> {
    use wavecrate::sample_sources::SourceDatabase;

    let old_relative = old_path
        .strip_prefix(source_root)
        .map_err(|_| String::from("Folder move metadata update failed: source folder mismatch"))?;
    let new_relative = new_path
        .strip_prefix(source_root)
        .map_err(|_| String::from("Folder move metadata update failed: target folder mismatch"))?;
    let db = SourceDatabase::open_for_user_metadata_write(source_root)
        .map_err(|err| format!("Folder move metadata update failed: {err}"))?;
    let entries = db
        .list_files_under_path(old_relative)
        .map_err(|err| format!("Folder move metadata update failed: {err}"))?;
    if entries.is_empty() {
        return Ok(());
    }

    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Folder move metadata update failed: {err}"))?;
    for entry in entries {
        let suffix = entry
            .relative_path
            .strip_prefix(old_relative)
            .map_err(|_| String::from("Folder move metadata update failed: invalid source row"))?;
        let target_relative = new_relative.join(suffix);
        batch
            .remap_wav_file_path(&entry.relative_path, &target_relative)
            .map_err(|err| format!("Folder move metadata update failed: {err}"))?;
        batch
            .remap_analysis_sample_identity(&entry.relative_path, &target_relative)
            .map_err(|err| format!("Folder move metadata update failed: {err}"))?;
    }
    batch
        .commit()
        .map_err(|err| format!("Folder move metadata update failed: {err}"))
}

pub(super) fn persist_moved_file_metadata(
    source_root: &Path,
    moves: &[(PathBuf, PathBuf)],
    remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
) -> Result<(), String> {
    use wavecrate::sample_sources::SourceDatabase;

    let remaps = moves
        .iter()
        .filter_map(|(old_path, new_path)| {
            let old_relative = old_path.strip_prefix(source_root).ok()?.to_path_buf();
            let new_relative = new_path.strip_prefix(source_root).ok()?.to_path_buf();
            Some((old_relative, new_relative))
        })
        .collect::<Vec<_>>();
    if remaps.is_empty() {
        return Ok(());
    }

    let db = SourceDatabase::open_for_user_metadata_write(source_root)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    for (old_relative, new_relative) in remaps {
        batch
            .remap_wav_file_path(&old_relative, &new_relative)
            .map_err(|err| format!("File move metadata update failed: {err}"))?;
        batch
            .remap_analysis_sample_identity(&old_relative, &new_relative)
            .map_err(|err| format!("File move metadata update failed: {err}"))?;
        if let Some(collection) = remove_from_collection {
            batch
                .remove_collection(&new_relative, collection)
                .map_err(|err| format!("File move metadata update failed: {err}"))?;
        }
    }
    batch
        .commit()
        .map_err(|err| format!("File move metadata update failed: {err}"))
}
