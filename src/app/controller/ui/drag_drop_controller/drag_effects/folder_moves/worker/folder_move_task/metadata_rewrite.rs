use super::result::rollback_and_error_result;
use super::*;
use crate::sample_sources::db::SourceWriteBatch;

/// Rewrite DB rows for all files now living under the moved folder.
pub(super) fn rewrite_folder_entries(
    db: &SourceDatabase,
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
    entries: &[FolderMoveEntry],
) -> Result<Vec<FolderEntryMove>, FolderMoveResult> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let mut batch = db.write_batch().map_err(|err| {
        rollback_and_error_result(
            request,
            prepared,
            format!("Failed to start database update: {err}"),
        )
    })?;
    let mut updates = Vec::with_capacity(entries.len());
    for entry in entries {
        updates.push(rewrite_entry(&mut batch, request, prepared, entry)?);
    }
    batch.commit().map_err(|err| {
        rollback_and_error_result(
            request,
            prepared,
            format!("Failed to save folder move: {err}"),
        )
    })?;
    Ok(updates)
}

/// Rewrite one DB row and mirror its metadata into the moved location.
fn rewrite_entry(
    batch: &mut SourceWriteBatch<'_>,
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
    moved_entry: &FolderMoveEntry,
) -> Result<FolderEntryMove, FolderMoveResult> {
    let entry = &moved_entry.entry;
    let suffix = entry
        .relative_path
        .strip_prefix(&request.folder)
        .unwrap_or_else(|_| Path::new(""));
    let updated_path = prepared.new_relative.join(suffix);
    batch.remove_file(&entry.relative_path).map_err(|err| {
        rollback_and_error_result(
            request,
            prepared,
            format!("Failed to drop old entry: {err}"),
        )
    })?;
    batch
        .upsert_file(&updated_path, entry.file_size, entry.modified_ns)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to register moved file: {err}"),
            )
        })?;
    copy_entry_metadata(batch, request, prepared, moved_entry, &updated_path)?;
    Ok(FolderEntryMove {
        old_relative: entry.relative_path.clone(),
        new_relative: updated_path,
        file_size: entry.file_size,
        modified_ns: entry.modified_ns,
        tag: entry.tag,
        looped: entry.looped,
        locked: entry.locked,
        last_played_at: entry.last_played_at,
        sound_type: entry.sound_type,
        user_tag: entry.user_tag.clone(),
        normal_tags: entry.normal_tags.clone(),
        collection: moved_entry.collection,
    })
}

fn copy_entry_metadata(
    batch: &mut SourceWriteBatch<'_>,
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
    moved_entry: &FolderMoveEntry,
    updated_path: &Path,
) -> Result<(), FolderMoveResult> {
    let entry = &moved_entry.entry;
    batch.set_tag(updated_path, entry.tag).map_err(|err| {
        rollback_and_error_result(request, prepared, format!("Failed to copy tag: {err}"))
    })?;
    batch
        .set_looped(updated_path, entry.looped)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy loop marker: {err}"),
            )
        })?;
    batch
        .set_locked(updated_path, entry.locked)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy keep lock: {err}"),
            )
        })?;
    if let Some(last_played_at) = entry.last_played_at {
        batch
            .set_last_played_at(updated_path, last_played_at)
            .map_err(|err| {
                rollback_and_error_result(
                    request,
                    prepared,
                    format!("Failed to copy playback age: {err}"),
                )
            })?;
    }
    if let Some(user_tag) = entry.user_tag.as_deref() {
        batch
            .set_user_tag(updated_path, Some(user_tag))
            .map_err(|err| {
                rollback_and_error_result(
                    request,
                    prepared,
                    format!("Failed to copy custom tag: {err}"),
                )
            })?;
    }
    if let Some(sound_type) = entry.sound_type {
        batch
            .set_sound_type(updated_path, Some(sound_type))
            .map_err(|err| {
                rollback_and_error_result(
                    request,
                    prepared,
                    format!("Failed to copy sound type: {err}"),
                )
            })?;
    }
    batch
        .replace_tags_for_path(updated_path, &entry.normal_tags)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy normal tags: {err}"),
            )
        })?;
    batch
        .set_collection(updated_path, moved_entry.collection)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy collection: {err}"),
            )
        })?;
    Ok(())
}
