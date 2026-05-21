//! Background job helpers for undo/redo file operations.

use crate::app::controller::jobs::{FileOpMessage, UndoFileJob, UndoFileOpResult, UndoFileOutcome};
use crate::app::controller::library::wav_io::file_metadata;
use crate::sample_sources::SourceDatabase;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
use std::time::Duration;

const REMOVE_SAMPLE_RETRY_ATTEMPTS: usize = 8;
const REMOVE_SAMPLE_RETRY_DELAY: Duration = Duration::from_millis(25);

/// Run an undo/redo filesystem job with optional progress updates.
pub(crate) fn run_undo_file_job(
    job: UndoFileJob,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> UndoFileOpResult {
    if cancel.load(Ordering::Relaxed) {
        return UndoFileOpResult {
            result: Err("Undo cancelled".to_string()),
            cancelled: true,
        };
    }
    let result = match job {
        UndoFileJob::Overwrite {
            source_id,
            source_root,
            relative_path,
            absolute_path,
            backup_path,
        } => {
            if let Some(parent) = absolute_path.parent()
                && let Err(err) = std::fs::create_dir_all(parent)
            {
                return UndoFileOpResult {
                    result: Err(format!(
                        "Failed to create folder {}: {err}",
                        parent.display()
                    )),
                    cancelled: false,
                };
            }
            std::fs::copy(&backup_path, &absolute_path)
                .map_err(|err| format!("Failed to restore audio: {err}"))
                .and_then(|_| {
                    let (file_size, modified_ns) = file_metadata(&absolute_path)?;
                    let db = SourceDatabase::open(&source_root)
                        .map_err(|err| format!("Database unavailable: {err}"))?;
                    let tag = db
                        .tag_for_path(&relative_path)
                        .map_err(|err| format!("Failed to read database: {err}"))?
                        .ok_or_else(|| "Sample not found in database".to_string())?;
                    let looped = db
                        .looped_for_path(&relative_path)
                        .map_err(|err| format!("Failed to read database: {err}"))?
                        .ok_or_else(|| "Sample not found in database".to_string())?;
                    let last_played_at = db
                        .last_played_at_for_path(&relative_path)
                        .map_err(|err| format!("Failed to read database: {err}"))?;
                    let normal_tags = db
                        .tag_labels_for_path(&relative_path)
                        .map_err(|err| format!("Failed to read database: {err}"))?;
                    Ok(UndoFileOutcome::Overwrite {
                        source_id,
                        relative_path,
                        file_size,
                        modified_ns,
                        tag,
                        looped,
                        last_played_at,
                        normal_tags,
                    })
                })
        }
        UndoFileJob::RemoveSample {
            source_id,
            source_root,
            relative_path,
            absolute_path,
        } => {
            let db = match SourceDatabase::open(&source_root) {
                Ok(db) => db,
                Err(err) => {
                    return UndoFileOpResult {
                        result: Err(format!("Database unavailable: {err}")),
                        cancelled: false,
                    };
                }
            };
            match remove_sample_file_with_retry(&absolute_path) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(err) => {
                    return UndoFileOpResult {
                        result: Err(format!(
                            "Failed to delete sample {}: {err}",
                            absolute_path.display()
                        )),
                        cancelled: false,
                    };
                }
            }
            if let Err(err) = db.remove_file(&relative_path) {
                return UndoFileOpResult {
                    result: Err(format!(
                        "Failed to drop database row for {}: {err}",
                        relative_path.display()
                    )),
                    cancelled: false,
                };
            }
            Ok(UndoFileOutcome::Removed {
                source_id,
                relative_path,
            })
        }
        UndoFileJob::RestoreSample {
            source_id,
            source_root,
            relative_path,
            absolute_path,
            backup_path,
            tag,
            looped,
            last_played_at,
            normal_tags,
        } => {
            if let Some(parent) = absolute_path.parent()
                && let Err(err) = std::fs::create_dir_all(parent)
            {
                return UndoFileOpResult {
                    result: Err(format!(
                        "Failed to create folder {}: {err}",
                        parent.display()
                    )),
                    cancelled: false,
                };
            }
            std::fs::copy(&backup_path, &absolute_path)
                .map_err(|err| format!("Failed to restore audio: {err}"))
                .and_then(|_| {
                    let (file_size, modified_ns) = file_metadata(&absolute_path)?;
                    let db = SourceDatabase::open(&source_root)
                        .map_err(|err| format!("Database unavailable: {err}"))?;
                    db.upsert_file(&relative_path, file_size, modified_ns)
                        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
                    db.set_tag(&relative_path, tag)
                        .map_err(|err| format!("Failed to sync tag: {err}"))?;
                    db.set_looped(&relative_path, looped)
                        .map_err(|err| format!("Failed to sync loop metadata: {err}"))?;
                    if let Some(last_played_at) = last_played_at {
                        db.set_last_played_at(&relative_path, last_played_at)
                            .map_err(|err| format!("Failed to sync playback age: {err}"))?;
                    } else {
                        db.clear_last_played_at(&relative_path)
                            .map_err(|err| format!("Failed to sync playback age: {err}"))?;
                    }
                    let mut batch = db
                        .write_batch()
                        .map_err(|err| format!("Failed to sync normal tags: {err}"))?;
                    batch
                        .replace_tags_for_path(&relative_path, &normal_tags)
                        .map_err(|err| format!("Failed to sync normal tags: {err}"))?;
                    batch
                        .commit()
                        .map_err(|err| format!("Failed to sync normal tags: {err}"))?;
                    Ok(UndoFileOutcome::Restored {
                        source_id,
                        relative_path,
                        file_size,
                        modified_ns,
                        tag,
                        looped,
                        last_played_at,
                        normal_tags,
                    })
                })
        }
    };
    if let Some(tx) = sender {
        let _ = tx.send(FileOpMessage::Progress {
            completed: 1,
            detail: None,
            item: None,
        });
    }
    UndoFileOpResult {
        result,
        cancelled: false,
    }
}

fn remove_sample_file_with_retry(path: &Path) -> std::io::Result<()> {
    for attempt in 0..REMOVE_SAMPLE_RETRY_ATTEMPTS {
        match std::fs::remove_file(path) {
            Err(err)
                if err.kind() == ErrorKind::PermissionDenied
                    && attempt + 1 < REMOVE_SAMPLE_RETRY_ATTEMPTS =>
            {
                std::thread::sleep(REMOVE_SAMPLE_RETRY_DELAY);
            }
            result => return result,
        }
    }
    unreachable!("retry loop always returns on its final attempt")
}
