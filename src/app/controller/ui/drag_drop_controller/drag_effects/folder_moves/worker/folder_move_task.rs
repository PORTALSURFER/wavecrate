//! Folder-level move worker implementation.

use super::report_progress;
use crate::app::controller::jobs::{
    FileOpMessage, FolderEntryMove, FolderMoveRequest, FolderMoveResult,
};
use crate::sample_sources::{SampleCollection, SourceDatabase, WavEntry};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

/// Precomputed filesystem paths for one folder move request.
struct PreparedFolderMove {
    new_relative: PathBuf,
    absolute_old: PathBuf,
    absolute_new: PathBuf,
}

/// Source DB row plus metadata that is not carried by `WavEntry`.
struct FolderMoveEntry {
    entry: WavEntry,
    collection: Option<SampleCollection>,
}

/// Prepared folder move transaction with explicit filesystem and database stages.
struct FolderMoveTransaction {
    request: FolderMoveRequest,
    prepared: PreparedFolderMove,
    db: SourceDatabase,
    entries: Vec<FolderMoveEntry>,
    moved: Vec<FolderEntryMove>,
}

/// Execute a background move for a folder dropped onto another folder.
pub(super) fn run_folder_move_task(
    request: FolderMoveRequest,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> FolderMoveResult {
    if cancel.load(Ordering::Relaxed) {
        return cancelled_result(&request);
    }

    let mut transaction = match prepare_folder_move_transaction(request) {
        Ok(transaction) => transaction,
        Err(result) => return result,
    };
    if let Err(result) = transaction.commit_filesystem_stage() {
        return result;
    }
    #[cfg(test)]
    super::run_before_folder_move_batch_hook();
    if let Err(result) = transaction.commit_db_stage() {
        return result;
    }
    report_progress(
        sender,
        1,
        Some(format!("Moved {}", transaction.request.folder.display())),
    );
    transaction.into_success()
}

/// Validate the request, open its source DB, and snapshot entries before mutation.
fn prepare_folder_move_transaction(
    request: FolderMoveRequest,
) -> Result<FolderMoveTransaction, FolderMoveResult> {
    let prepared = prepare_folder_move(&request)?;
    let db = SourceDatabase::open(&request.source_root).map_err(|err| {
        error_result(
            &request,
            prepared.new_relative.clone(),
            format!("Failed to open source DB: {err}"),
            false,
        )
    })?;
    let entries = load_folder_entries(&db, &request, &prepared)?;
    Ok(FolderMoveTransaction {
        request,
        prepared,
        db,
        entries,
        moved: Vec::new(),
    })
}

impl FolderMoveTransaction {
    /// Rename the folder before database rows are rewritten.
    fn commit_filesystem_stage(&self) -> Result<(), FolderMoveResult> {
        rename_folder(&self.request, &self.prepared)
    }

    /// Rewrite all tracked rows or roll the folder rename back on failure.
    fn commit_db_stage(&mut self) -> Result<(), FolderMoveResult> {
        self.moved =
            rewrite_folder_entries(&self.db, &self.request, &self.prepared, &self.entries)?;
        Ok(())
    }

    /// Build the standard success payload after both transaction stages commit.
    fn into_success(self) -> FolderMoveResult {
        success_result(&self.request, self.prepared.new_relative, self.moved)
    }
}

/// Validate the request and derive the old/new filesystem locations.
fn prepare_folder_move(
    request: &FolderMoveRequest,
) -> Result<PreparedFolderMove, FolderMoveResult> {
    if request.folder.as_os_str().is_empty() {
        return Err(error_result(
            request,
            request.target_folder.clone(),
            "Root folder cannot be moved",
            false,
        ));
    }
    if request.target_folder == request.folder {
        return Err(error_result(
            request,
            request.target_folder.clone(),
            "Folder is already there",
            false,
        ));
    }
    if request.target_folder.starts_with(&request.folder) {
        return Err(error_result(
            request,
            request.target_folder.clone(),
            "Cannot move a folder into itself",
            false,
        ));
    }
    let folder_name = match request.folder.file_name() {
        Some(name) => name.to_owned(),
        None => {
            return Err(error_result(
                request,
                request.target_folder.clone(),
                "Folder name unavailable for move",
                false,
            ));
        }
    };
    let new_relative = if request.target_folder.as_os_str().is_empty() {
        PathBuf::from(folder_name)
    } else {
        request.target_folder.join(folder_name)
    };
    let absolute_old = request.source_root.join(&request.folder);
    if !absolute_old.is_dir() {
        return Err(error_result(
            request,
            new_relative,
            format!("Folder not found: {}", request.folder.display()),
            false,
        ));
    }
    if !request.target_folder.as_os_str().is_empty() {
        let destination_dir = request.source_root.join(&request.target_folder);
        if !destination_dir.is_dir() {
            return Err(error_result(
                request,
                new_relative,
                format!("Folder not found: {}", request.target_folder.display()),
                false,
            ));
        }
    }
    let absolute_new = request.source_root.join(&new_relative);
    if absolute_new.exists() {
        let message = format!("Folder already exists: {}", new_relative.display());
        return Err(error_result(request, new_relative, message, false));
    }
    Ok(PreparedFolderMove {
        new_relative,
        absolute_old,
        absolute_new,
    })
}

/// Load source DB rows that need their relative path rewritten after the move.
fn load_folder_entries(
    db: &SourceDatabase,
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
) -> Result<Vec<FolderMoveEntry>, FolderMoveResult> {
    let entries = db.list_files().map_err(|err| {
        error_result(
            request,
            prepared.new_relative.clone(),
            format!("Failed to list folder entries: {err}"),
            false,
        )
    })?;
    let mut folder_entries = Vec::new();
    for entry in entries
        .into_iter()
        .filter(|entry| entry.relative_path.starts_with(&request.folder))
    {
        let collection = db
            .collection_for_path(&entry.relative_path)
            .map_err(|err| {
                error_result(
                    request,
                    prepared.new_relative.clone(),
                    format!(
                        "Failed to read collection for {}: {err}",
                        entry.relative_path.display()
                    ),
                    false,
                )
            })?;
        folder_entries.push(FolderMoveEntry { entry, collection });
    }
    Ok(folder_entries)
}

/// Rename the folder on disk before the DB rewrite phase begins.
fn rename_folder(
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
) -> Result<(), FolderMoveResult> {
    std::fs::rename(&prepared.absolute_old, &prepared.absolute_new).map_err(|err| {
        error_result(
            request,
            prepared.new_relative.clone(),
            format!("Failed to move folder: {err}"),
            false,
        )
    })
}

/// Rewrite DB rows for all files now living under the moved folder.
fn rewrite_folder_entries(
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
    batch: &mut crate::sample_sources::db::SourceWriteBatch<'_>,
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
    batch.set_tag(&updated_path, entry.tag).map_err(|err| {
        rollback_and_error_result(request, prepared, format!("Failed to copy tag: {err}"))
    })?;
    batch
        .set_looped(&updated_path, entry.looped)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy loop marker: {err}"),
            )
        })?;
    batch
        .set_locked(&updated_path, entry.locked)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy keep lock: {err}"),
            )
        })?;
    if let Some(last_played_at) = entry.last_played_at {
        batch
            .set_last_played_at(&updated_path, last_played_at)
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
            .set_user_tag(&updated_path, Some(user_tag))
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
            .set_sound_type(&updated_path, Some(sound_type))
            .map_err(|err| {
                rollback_and_error_result(
                    request,
                    prepared,
                    format!("Failed to copy sound type: {err}"),
                )
            })?;
    }
    batch
        .replace_tags_for_path(&updated_path, &entry.normal_tags)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy normal tags: {err}"),
            )
        })?;
    batch
        .set_collection(&updated_path, moved_entry.collection)
        .map_err(|err| {
            rollback_and_error_result(
                request,
                prepared,
                format!("Failed to copy collection: {err}"),
            )
        })?;
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

/// Roll the filesystem rename back after a DB failure and return a failed result.
fn rollback_and_error_result(
    request: &FolderMoveRequest,
    prepared: &PreparedFolderMove,
    message: String,
) -> FolderMoveResult {
    let _ = std::fs::rename(&prepared.absolute_new, &prepared.absolute_old);
    error_result(request, prepared.new_relative.clone(), message, false)
}

/// Return the standard cancelled result payload for folder moves.
fn cancelled_result(request: &FolderMoveRequest) -> FolderMoveResult {
    FolderMoveResult {
        source_id: request.source_id.clone(),
        old_folder: request.folder.clone(),
        new_folder: request.target_folder.clone(),
        folder_moved: false,
        moved: Vec::new(),
        errors: Vec::new(),
        cancelled: true,
    }
}

/// Return a failed result payload with one error message.
fn error_result(
    request: &FolderMoveRequest,
    new_folder: PathBuf,
    message: impl Into<String>,
    folder_moved: bool,
) -> FolderMoveResult {
    FolderMoveResult {
        source_id: request.source_id.clone(),
        old_folder: request.folder.clone(),
        new_folder,
        folder_moved,
        moved: Vec::new(),
        errors: vec![message.into()],
        cancelled: false,
    }
}

/// Return a successful result payload after the move and DB rewrite both complete.
fn success_result(
    request: &FolderMoveRequest,
    new_folder: PathBuf,
    moved: Vec<FolderEntryMove>,
) -> FolderMoveResult {
    FolderMoveResult {
        source_id: request.source_id.clone(),
        old_folder: request.folder.clone(),
        new_folder,
        folder_moved: true,
        moved,
        errors: Vec::new(),
        cancelled: false,
    }
}
