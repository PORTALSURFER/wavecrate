use super::super::super::file_metadata;
use super::journal::{remove_folder_move_journal_entry, rollback_folder_move_to_source};
use crate::app::controller::jobs::{
    FileOpMessage, FolderEntryMove, FolderMoveRequest, FolderMoveResult, FolderSampleMoveRequest,
    FolderSampleMoveResult,
};
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{SourceDatabase, SourceId};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

/// Execute a background batch of sample moves inside one source folder.
pub(super) fn run_folder_sample_move_task(
    source_id: SourceId,
    source_root: PathBuf,
    requests: Vec<FolderSampleMoveRequest>,
    mut errors: Vec<String>,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> FolderSampleMoveResult {
    let mut moved = Vec::new();
    let mut completed = 0usize;
    let mut cancelled = false;
    if !source_root.is_dir() {
        errors.push(format!("Source folder missing: {}", source_root.display()));
        return FolderSampleMoveResult {
            source_id,
            moved,
            errors,
            cancelled,
        };
    }
    let db = match SourceDatabase::open(&source_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(format!("Failed to open source DB: {err}"));
            return FolderSampleMoveResult {
                source_id,
                moved,
                errors,
                cancelled,
            };
        }
    };
    for request in requests {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let detail = Some(format!("Moving {}", request.relative_path.display()));
        let absolute = source_root.join(&request.relative_path);
        if !absolute.is_file() {
            errors.push(format!("File missing: {}", request.relative_path.display()));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Some(parent) = request.target_relative.parent() {
            let target_dir = source_root.join(parent);
            if !target_dir.is_dir() {
                errors.push(format!("Folder not found: {}", parent.display()));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        }
        let target_absolute = source_root.join(&request.target_relative);
        if target_absolute.exists() {
            errors.push(format!(
                "A file already exists at {}",
                request.target_relative.display()
            ));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        let tag = match db.tag_for_path(&request.relative_path) {
            Ok(Some(tag)) => tag,
            Ok(None) => {
                errors.push("Sample not found in database".to_string());
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
            Err(err) => {
                errors.push(format!("Failed to read database: {err}"));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        let looped = match db.looped_for_path(&request.relative_path) {
            Ok(Some(looped)) => looped,
            Ok(None) => {
                errors.push("Sample not found in database".to_string());
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
            Err(err) => {
                errors.push(format!("Failed to read database: {err}"));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        let last_played_at = match db.last_played_at_for_path(&request.relative_path) {
            Ok(value) => value,
            Err(err) => {
                errors.push(format!("Failed to read database: {err}"));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        let op_id = file_ops_journal::new_op_id();
        let staged_relative =
            match file_ops_journal::staged_relative_for_target(&request.target_relative, &op_id) {
                Ok(path) => path,
                Err(err) => {
                    errors.push(format!("Failed to build staging path: {err}"));
                    completed += 1;
                    report_progress(sender, completed, detail);
                    continue;
                }
            };
        let journal_entry = match file_ops_journal::FileOpJournalEntry::new_move(
            op_id.clone(),
            source_root.clone(),
            request.relative_path.clone(),
            request.target_relative.clone(),
            staged_relative.clone(),
            tag,
            looped,
            last_played_at,
        ) {
            Ok(entry) => entry,
            Err(err) => {
                errors.push(format!("Failed to stage move journal: {err}"));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        if let Err(err) = file_ops_journal::insert_entry(&db, &journal_entry) {
            errors.push(format!("Failed to record move journal: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        let staged_absolute = source_root.join(&staged_relative);
        if let Err(err) = std::fs::rename(&absolute, &staged_absolute) {
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to move file: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        let (file_size, modified_ns) = match file_metadata(&staged_absolute) {
            Ok(meta) => meta,
            Err(err) => {
                rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
                remove_folder_move_journal_entry(&mut errors, &db, &op_id);
                errors.push(err);
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        if let Err(err) = file_ops_journal::update_stage(
            &db,
            &op_id,
            file_ops_journal::FileOpStage::Staged,
            Some(file_size),
            Some(modified_ns),
        ) {
            rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to update move journal: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        let mut batch = match db.write_batch() {
            Ok(batch) => batch,
            Err(err) => {
                rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
                remove_folder_move_journal_entry(&mut errors, &db, &op_id);
                errors.push(format!("Failed to start database update: {err}"));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        if let Err(err) = batch.remove_file(&request.relative_path) {
            rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to drop old entry: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.upsert_file(&request.target_relative, file_size, modified_ns) {
            rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to register moved file: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.set_tag(&request.target_relative, tag) {
            rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to copy tag: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.set_looped(&request.target_relative, looped) {
            rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to copy loop marker: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Some(last_played_at) = last_played_at
            && let Err(err) = batch.set_last_played_at(&request.target_relative, last_played_at)
        {
            rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to copy playback age: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.commit() {
            rollback_folder_move_to_source(&mut errors, &staged_absolute, &absolute);
            remove_folder_move_journal_entry(&mut errors, &db, &op_id);
            errors.push(format!("Failed to save move: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = file_ops_journal::update_stage(
            &db,
            &op_id,
            file_ops_journal::FileOpStage::TargetDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
        if let Err(err) = file_ops_journal::update_stage(
            &db,
            &op_id,
            file_ops_journal::FileOpStage::SourceDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
        if let Err(err) = std::fs::rename(&staged_absolute, &target_absolute) {
            errors.push(format!("Failed to finalize move: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        remove_folder_move_journal_entry(&mut errors, &db, &op_id);
        moved.push(FolderEntryMove {
            old_relative: request.relative_path,
            new_relative: request.target_relative,
            file_size,
            modified_ns,
            tag,
            looped,
            last_played_at,
        });
        completed += 1;
        report_progress(sender, completed, detail);
    }
    FolderSampleMoveResult {
        source_id,
        moved,
        errors,
        cancelled,
    }
}

/// Execute a background move for a folder dropped onto another folder.
pub(super) fn run_folder_move_task(
    request: FolderMoveRequest,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> FolderMoveResult {
    let mut errors = Vec::new();
    let mut moved = Vec::new();
    let mut folder_moved = false;
    let mut cancelled = false;
    if cancel.load(Ordering::Relaxed) {
        cancelled = true;
        return FolderMoveResult {
            source_id: request.source_id,
            old_folder: request.folder,
            new_folder: request.target_folder,
            folder_moved,
            moved,
            errors,
            cancelled,
        };
    }
    if request.folder.as_os_str().is_empty() {
        errors.push("Root folder cannot be moved".to_string());
        return FolderMoveResult {
            source_id: request.source_id,
            old_folder: request.folder,
            new_folder: request.target_folder,
            folder_moved,
            moved,
            errors,
            cancelled,
        };
    }
    if request.target_folder == request.folder {
        errors.push("Folder is already there".to_string());
        return FolderMoveResult {
            source_id: request.source_id,
            old_folder: request.folder,
            new_folder: request.target_folder,
            folder_moved,
            moved,
            errors,
            cancelled,
        };
    }
    if request.target_folder.starts_with(&request.folder) {
        errors.push("Cannot move a folder into itself".to_string());
        return FolderMoveResult {
            source_id: request.source_id,
            old_folder: request.folder,
            new_folder: request.target_folder,
            folder_moved,
            moved,
            errors,
            cancelled,
        };
    }
    let folder_name = match request.folder.file_name() {
        Some(name) => name.to_owned(),
        None => {
            errors.push("Folder name unavailable for move".to_string());
            return FolderMoveResult {
                source_id: request.source_id,
                old_folder: request.folder,
                new_folder: request.target_folder,
                folder_moved,
                moved,
                errors,
                cancelled,
            };
        }
    };
    let new_relative = if request.target_folder.as_os_str().is_empty() {
        PathBuf::from(folder_name)
    } else {
        request.target_folder.join(folder_name)
    };
    let absolute_old = request.source_root.join(&request.folder);
    if !absolute_old.is_dir() {
        errors.push(format!("Folder not found: {}", request.folder.display()));
        return FolderMoveResult {
            source_id: request.source_id,
            old_folder: request.folder,
            new_folder: new_relative,
            folder_moved,
            moved,
            errors,
            cancelled,
        };
    }
    if !request.target_folder.as_os_str().is_empty() {
        let destination_dir = request.source_root.join(&request.target_folder);
        if !destination_dir.is_dir() {
            errors.push(format!(
                "Folder not found: {}",
                request.target_folder.display()
            ));
            return FolderMoveResult {
                source_id: request.source_id,
                old_folder: request.folder,
                new_folder: new_relative,
                folder_moved,
                moved,
                errors,
                cancelled,
            };
        }
    }
    let absolute_new = request.source_root.join(&new_relative);
    if absolute_new.exists() {
        errors.push(format!("Folder already exists: {}", new_relative.display()));
        return FolderMoveResult {
            source_id: request.source_id,
            old_folder: request.folder,
            new_folder: new_relative,
            folder_moved,
            moved,
            errors,
            cancelled,
        };
    }
    let db = match SourceDatabase::open(&request.source_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(format!("Failed to open source DB: {err}"));
            return FolderMoveResult {
                source_id: request.source_id,
                old_folder: request.folder,
                new_folder: new_relative,
                folder_moved,
                moved,
                errors,
                cancelled,
            };
        }
    };
    let entries = match db.list_files() {
        Ok(entries) => entries
            .into_iter()
            .filter(|entry| entry.relative_path.starts_with(&request.folder))
            .collect::<Vec<_>>(),
        Err(err) => {
            errors.push(format!("Failed to list folder entries: {err}"));
            return FolderMoveResult {
                source_id: request.source_id,
                old_folder: request.folder,
                new_folder: new_relative,
                folder_moved,
                moved,
                errors,
                cancelled,
            };
        }
    };
    if let Err(err) = std::fs::rename(&absolute_old, &absolute_new) {
        errors.push(format!("Failed to move folder: {err}"));
        return FolderMoveResult {
            source_id: request.source_id,
            old_folder: request.folder,
            new_folder: new_relative,
            folder_moved,
            moved,
            errors,
            cancelled,
        };
    }
    folder_moved = true;
    if !entries.is_empty() {
        let mut updates = Vec::with_capacity(entries.len());
        let mut batch = match db.write_batch() {
            Ok(batch) => batch,
            Err(err) => {
                let _ = std::fs::rename(&absolute_new, &absolute_old);
                errors.push(format!("Failed to start database update: {err}"));
                return FolderMoveResult {
                    source_id: request.source_id,
                    old_folder: request.folder,
                    new_folder: new_relative,
                    folder_moved: false,
                    moved,
                    errors,
                    cancelled,
                };
            }
        };
        for entry in &entries {
            let suffix = entry
                .relative_path
                .strip_prefix(&request.folder)
                .unwrap_or_else(|_| Path::new(""));
            let updated_path = new_relative.join(suffix);
            if let Err(err) = batch.remove_file(&entry.relative_path) {
                let _ = std::fs::rename(&absolute_new, &absolute_old);
                errors.push(format!("Failed to drop old entry: {err}"));
                return FolderMoveResult {
                    source_id: request.source_id,
                    old_folder: request.folder,
                    new_folder: new_relative,
                    folder_moved: false,
                    moved,
                    errors,
                    cancelled,
                };
            }
            if let Err(err) = batch.upsert_file(&updated_path, entry.file_size, entry.modified_ns) {
                let _ = std::fs::rename(&absolute_new, &absolute_old);
                errors.push(format!("Failed to register moved file: {err}"));
                return FolderMoveResult {
                    source_id: request.source_id,
                    old_folder: request.folder,
                    new_folder: new_relative,
                    folder_moved: false,
                    moved,
                    errors,
                    cancelled,
                };
            }
            if let Err(err) = batch.set_tag(&updated_path, entry.tag) {
                let _ = std::fs::rename(&absolute_new, &absolute_old);
                errors.push(format!("Failed to copy tag: {err}"));
                return FolderMoveResult {
                    source_id: request.source_id,
                    old_folder: request.folder,
                    new_folder: new_relative,
                    folder_moved: false,
                    moved,
                    errors,
                    cancelled,
                };
            }
            if let Some(last_played_at) = entry.last_played_at
                && let Err(err) = batch.set_last_played_at(&updated_path, last_played_at)
            {
                let _ = std::fs::rename(&absolute_new, &absolute_old);
                errors.push(format!("Failed to copy playback age: {err}"));
                return FolderMoveResult {
                    source_id: request.source_id,
                    old_folder: request.folder,
                    new_folder: new_relative,
                    folder_moved: false,
                    moved,
                    errors,
                    cancelled,
                };
            }
            updates.push(FolderEntryMove {
                old_relative: entry.relative_path.clone(),
                new_relative: updated_path,
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                tag: entry.tag,
                looped: entry.looped,
                last_played_at: entry.last_played_at,
            });
        }
        if let Err(err) = batch.commit() {
            let _ = std::fs::rename(&absolute_new, &absolute_old);
            errors.push(format!("Failed to save folder move: {err}"));
            return FolderMoveResult {
                source_id: request.source_id,
                old_folder: request.folder,
                new_folder: new_relative,
                folder_moved: false,
                moved,
                errors,
                cancelled,
            };
        }
        moved = updates;
    }
    report_progress(
        sender,
        1,
        Some(format!("Moved {}", request.folder.display())),
    );
    FolderMoveResult {
        source_id: request.source_id,
        old_folder: request.folder,
        new_folder: new_relative,
        folder_moved,
        moved,
        errors,
        cancelled,
    }
}

fn report_progress(
    sender: Option<&Sender<FileOpMessage>>,
    completed: usize,
    detail: Option<String>,
) {
    if let Some(tx) = sender {
        let _ = tx.send(FileOpMessage::Progress { completed, detail });
    }
}
