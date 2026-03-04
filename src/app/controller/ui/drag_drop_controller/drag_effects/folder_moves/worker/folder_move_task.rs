//! Folder-level move worker implementation.

use super::report_progress;
use crate::app::controller::jobs::{
    FileOpMessage, FolderEntryMove, FolderMoveRequest, FolderMoveResult,
};
use crate::sample_sources::SourceDatabase;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

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
