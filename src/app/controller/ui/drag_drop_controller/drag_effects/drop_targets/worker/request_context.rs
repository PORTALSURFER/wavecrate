use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::super::super::move_transaction::load_sample_move_metadata;
use super::super::DroppedSampleMetadata;
use crate::app::controller::jobs::DropTargetTransferMetadata;
use crate::sample_sources::SourceDatabase;

pub(super) fn drop_target_metadata_from_request(
    metadata: DropTargetTransferMetadata,
) -> DroppedSampleMetadata {
    DroppedSampleMetadata {
        tag: metadata.tag,
        looped: metadata.looped,
        locked: metadata.locked,
        last_played_at: metadata.last_played_at,
        last_curated_at: metadata.last_curated_at,
        sound_type: metadata.sound_type,
        user_tag: metadata.user_tag,
        normal_tags: metadata.normal_tags,
        collection: metadata.collection,
    }
}

pub(super) fn source_db_for<'a>(
    target_root: &Path,
    target_db: &'a SourceDatabase,
    source_dbs: &'a mut HashMap<PathBuf, SourceDatabase>,
    source_root: &Path,
) -> Result<&'a SourceDatabase, String> {
    if source_root == target_root {
        return Ok(target_db);
    }
    if !source_dbs.contains_key(source_root) {
        let db = SourceDatabase::open(source_root)
            .map_err(|err| format!("Failed to open source DB: {err}"))?;
        source_dbs.insert(source_root.to_path_buf(), db);
    }
    source_dbs
        .get(source_root)
        .ok_or_else(|| "Source database unavailable".to_string())
}

pub(super) fn load_dropped_sample_metadata(
    db: &SourceDatabase,
    relative_path: &Path,
) -> Result<DroppedSampleMetadata, String> {
    let metadata = load_sample_move_metadata(db, relative_path)?;
    Ok(DroppedSampleMetadata {
        tag: metadata.tag,
        looped: metadata.looped,
        locked: metadata.locked,
        last_played_at: metadata.last_played_at,
        last_curated_at: metadata.last_curated_at,
        sound_type: metadata.sound_type,
        user_tag: metadata.user_tag,
        normal_tags: metadata.normal_tags,
        collection: metadata.collection,
    })
}

pub(super) fn file_name_for_request(relative_path: &Path) -> Result<OsString, String> {
    relative_path
        .file_name()
        .map(|name| name.to_owned())
        .ok_or_else(|| "Sample name unavailable for drop".to_string())
}
