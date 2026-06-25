use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use super::{
    FileMoveItem, FolderBrowserState,
    path_helpers::path_id,
    scanning::{file_entry_for_source_path, upsert_file},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourcedMovedFile {
    pub(super) source_root: PathBuf,
    pub(super) old_path: PathBuf,
    pub(super) new_path: PathBuf,
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
        let target_parent_id = path_id(target_parent);
        let old_ids = moves
            .iter()
            .map(|move_item| path_id(&move_item.old_path))
            .collect::<HashSet<_>>();
        let previous_files = self.previous_files_for_moves(&old_ids);

        for source_index in 0..self.source.sources.len() {
            let source_root = self.source.sources[source_index].root.clone();
            let source_old_ids = moves
                .iter()
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
            let mut moved_file =
                file_entry_for_source_path(&move_item.new_path, &target_source_root);
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
                if move_item.old_path.starts_with(&source.root) {
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
            self.rewrite_similarity_path_prefix(&move_item.old_path, &move_item.new_path);
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
        .map(|item| (PathBuf::from(&item.file_id), item.source_root.clone()))
        .collect::<HashMap<_, _>>();
    moved_paths
        .iter()
        .filter_map(|(old_path, new_path)| {
            source_roots
                .get(old_path)
                .cloned()
                .map(|source_root| SourcedMovedFile {
                    source_root,
                    old_path: old_path.clone(),
                    new_path: new_path.clone(),
                })
        })
        .collect()
}

pub(super) fn persist_sourced_moved_file_metadata(
    target_source_root: &Path,
    moves: &[SourcedMovedFile],
    remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    let same_source_moves = moves
        .iter()
        .filter(|move_item| move_item.source_root == target_source_root)
        .map(|move_item| (move_item.old_path.clone(), move_item.new_path.clone()))
        .collect::<Vec<_>>();
    if !same_source_moves.is_empty()
        && let Err(error) = super::drag_drop_relocation::persist_moved_file_metadata(
            target_source_root,
            &same_source_moves,
            remove_from_collection,
        )
    {
        errors.push(error);
    }

    for move_item in moves
        .iter()
        .filter(|move_item| move_item.source_root != target_source_root)
    {
        if let Err(error) = persist_cross_source_moved_file_metadata(
            &move_item.source_root,
            target_source_root,
            &move_item.old_path,
            &move_item.new_path,
            remove_from_collection,
        ) {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn persist_cross_source_moved_file_metadata(
    source_root: &Path,
    target_source_root: &Path,
    old_path: &Path,
    new_path: &Path,
    remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
) -> Result<(), String> {
    use wavecrate::sample_sources::SourceDatabase;

    let old_relative = old_path
        .strip_prefix(source_root)
        .map_err(|_| String::from("File move metadata update failed: source file mismatch"))?;
    let new_relative = new_path
        .strip_prefix(target_source_root)
        .map_err(|_| String::from("File move metadata update failed: target file mismatch"))?;
    let source_db = SourceDatabase::open_for_user_metadata_write(source_root)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    let target_db = SourceDatabase::open_for_user_metadata_write(target_source_root)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    let metadata = moved_file_metadata(&source_db, old_relative)?;
    write_cross_source_target_metadata(
        &target_db,
        new_relative,
        new_path,
        metadata.as_ref(),
        remove_from_collection,
    )?;

    let mut source_batch = source_db
        .write_batch()
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    source_batch
        .remove_file(old_relative)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    source_batch
        .commit()
        .map_err(|err| format!("File move metadata update failed: {err}"))
}

struct MovedFileMetadata {
    entry: wavecrate::sample_sources::WavEntry,
    normal_tags: Vec<String>,
    collections: Vec<wavecrate::sample_sources::SampleCollection>,
}

fn moved_file_metadata(
    db: &wavecrate::sample_sources::SourceDatabase,
    relative_path: &Path,
) -> Result<Option<MovedFileMetadata>, String> {
    let Some(entry) = db
        .entry_for_path(relative_path)
        .map_err(|err| format!("File move metadata update failed: {err}"))?
    else {
        return Ok(None);
    };
    let normal_tags = db
        .tag_labels_for_path(relative_path)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    let collections = db
        .collections_for_path(relative_path)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    Ok(Some(MovedFileMetadata {
        entry,
        normal_tags,
        collections,
    }))
}

fn write_cross_source_target_metadata(
    db: &wavecrate::sample_sources::SourceDatabase,
    relative_path: &Path,
    absolute_path: &Path,
    metadata: Option<&MovedFileMetadata>,
    remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
) -> Result<(), String> {
    let (file_size, modified_ns) = file_metadata(absolute_path)?;
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    match metadata.and_then(|metadata| metadata.entry.content_hash.as_deref()) {
        Some(content_hash) => batch
            .upsert_file_with_hash(relative_path, file_size, modified_ns, content_hash)
            .map_err(|err| format!("File move metadata update failed: {err}"))?,
        None => batch
            .upsert_file_without_hash(relative_path, file_size, modified_ns)
            .map_err(|err| format!("File move metadata update failed: {err}"))?,
    }
    if let Some(metadata) = metadata {
        write_cross_source_user_metadata(
            &mut batch,
            relative_path,
            metadata,
            remove_from_collection,
        )?;
    }
    batch
        .commit()
        .map_err(|err| format!("File move metadata update failed: {err}"))
}

fn write_cross_source_user_metadata(
    batch: &mut wavecrate::sample_sources::db::SourceWriteBatch<'_>,
    relative_path: &Path,
    metadata: &MovedFileMetadata,
    remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
) -> Result<(), String> {
    batch
        .set_tag(relative_path, metadata.entry.tag)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    batch
        .set_looped(relative_path, metadata.entry.looped)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    batch
        .set_locked(relative_path, metadata.entry.locked)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    batch
        .set_sound_type(relative_path, metadata.entry.sound_type)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    batch
        .set_user_tag(relative_path, metadata.entry.user_tag.as_deref())
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    batch
        .set_tag_named(relative_path, metadata.entry.tag_named)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    match metadata.entry.last_played_at {
        Some(last_played_at) => batch
            .set_last_played_at(relative_path, last_played_at)
            .map_err(|err| format!("File move metadata update failed: {err}"))?,
        None => batch
            .clear_last_played_at(relative_path)
            .map_err(|err| format!("File move metadata update failed: {err}"))?,
    }
    batch
        .replace_tags_for_path(relative_path, &metadata.normal_tags)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    batch
        .set_collection(relative_path, None)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    for collection in &metadata.collections {
        batch
            .add_collection(relative_path, *collection)
            .map_err(|err| format!("File move metadata update failed: {err}"))?;
    }
    if let Some(collection) = remove_from_collection {
        batch
            .remove_collection(relative_path, collection)
            .map_err(|err| format!("File move metadata update failed: {err}"))?;
    }
    match metadata.entry.last_curated_at {
        Some(last_curated_at) => batch
            .set_last_curated_at(relative_path, last_curated_at)
            .map_err(|err| format!("File move metadata update failed: {err}"))?,
        None => batch
            .clear_last_curated_at(relative_path)
            .map_err(|err| format!("File move metadata update failed: {err}"))?,
    }
    Ok(())
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("File move metadata update failed: {err}"))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("File move metadata update failed: {err}"))?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| String::from("File move metadata update failed: modified time before epoch"))?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}
