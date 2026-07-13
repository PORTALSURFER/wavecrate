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
        return undo_job_cancelled();
    }
    let result = run_undo_file_job_inner(job);
    report_undo_file_progress(sender);
    UndoFileOpResult {
        result,
        cancelled: false,
    }
}

fn undo_job_cancelled() -> UndoFileOpResult {
    UndoFileOpResult {
        result: Err("Undo cancelled".to_string()),
        cancelled: true,
    }
}

fn run_undo_file_job_inner(job: UndoFileJob) -> Result<UndoFileOutcome, String> {
    match job {
        UndoFileJob::Overwrite {
            source_id,
            source_root,
            relative_path,
            absolute_path,
            backup_path,
        } => restore_overwrite_sample(
            source_id,
            source_root.as_path(),
            relative_path.as_path(),
            absolute_path.as_path(),
            backup_path.as_path(),
        ),
        UndoFileJob::RemoveSample {
            source_id,
            source_root,
            relative_path,
            absolute_path,
        } => remove_restored_sample(
            source_id,
            source_root.as_path(),
            relative_path.as_path(),
            absolute_path.as_path(),
        ),
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
        } => restore_removed_sample(RestoreSampleInput {
            source_id,
            source_root: source_root.as_path(),
            relative_path: relative_path.as_path(),
            absolute_path: absolute_path.as_path(),
            backup_path: backup_path.as_path(),
            tag,
            looped,
            last_played_at,
            normal_tags,
        }),
    }
}

fn report_undo_file_progress(sender: Option<&Sender<FileOpMessage>>) {
    if let Some(tx) = sender {
        let _ = tx.send(FileOpMessage::Progress {
            completed: 1,
            detail: None,
            item: None,
        });
    }
}

fn restore_overwrite_sample(
    source_id: crate::sample_sources::SourceId,
    source_root: &Path,
    relative_path: &Path,
    absolute_path: &Path,
    backup_path: &Path,
) -> Result<UndoFileOutcome, String> {
    ensure_parent_dir(absolute_path)?;
    std::fs::copy(backup_path, absolute_path)
        .map_err(|err| format!("Failed to restore audio: {err}"))?;

    let (file_size, modified_ns) = file_metadata(absolute_path)?;
    let db = open_source_db(source_root)?;
    let snapshot = read_sample_metadata(&db, relative_path)?;

    Ok(UndoFileOutcome::Overwrite {
        source_id,
        relative_path: relative_path.to_path_buf(),
        file_size,
        modified_ns,
        tag: snapshot.tag,
        looped: snapshot.looped,
        last_played_at: snapshot.last_played_at,
        normal_tags: snapshot.normal_tags,
    })
}

fn remove_restored_sample(
    source_id: crate::sample_sources::SourceId,
    source_root: &Path,
    relative_path: &Path,
    absolute_path: &Path,
) -> Result<UndoFileOutcome, String> {
    let db = open_source_db(source_root)?;
    delete_sample_file_if_present(absolute_path)?;
    db.remove_file(relative_path).map_err(|err| {
        format!(
            "Failed to drop database row for {}: {err}",
            relative_path.display()
        )
    })?;

    Ok(UndoFileOutcome::Removed {
        source_id,
        relative_path: relative_path.to_path_buf(),
    })
}

struct RestoreSampleInput<'a> {
    source_id: crate::sample_sources::SourceId,
    source_root: &'a Path,
    relative_path: &'a Path,
    absolute_path: &'a Path,
    backup_path: &'a Path,
    tag: crate::sample_sources::Rating,
    looped: bool,
    last_played_at: Option<i64>,
    normal_tags: Vec<String>,
}

fn restore_removed_sample(input: RestoreSampleInput<'_>) -> Result<UndoFileOutcome, String> {
    ensure_parent_dir(input.absolute_path)?;
    std::fs::copy(input.backup_path, input.absolute_path)
        .map_err(|err| format!("Failed to restore audio: {err}"))?;

    let (file_size, modified_ns) = file_metadata(input.absolute_path)?;
    let db = open_source_db(input.source_root)?;
    write_restored_sample_metadata(&db, &input, file_size, modified_ns)?;

    Ok(UndoFileOutcome::Restored {
        source_id: input.source_id,
        relative_path: input.relative_path.to_path_buf(),
        file_size,
        modified_ns,
        tag: input.tag,
        looped: input.looped,
        last_played_at: input.last_played_at,
        normal_tags: input.normal_tags,
    })
}

struct SampleMetadataSnapshot {
    tag: crate::sample_sources::Rating,
    looped: bool,
    last_played_at: Option<i64>,
    normal_tags: Vec<String>,
}

fn read_sample_metadata(
    db: &SourceDatabase,
    relative_path: &Path,
) -> Result<SampleMetadataSnapshot, String> {
    let tag = db
        .tag_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))?
        .ok_or_else(|| "Sample not found in database".to_string())?;
    let looped = db
        .looped_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))?
        .ok_or_else(|| "Sample not found in database".to_string())?;
    let last_played_at = db
        .last_played_at_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))?;
    let normal_tags = db
        .tag_labels_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))?;

    Ok(SampleMetadataSnapshot {
        tag,
        looped,
        last_played_at,
        normal_tags,
    })
}

fn write_restored_sample_metadata(
    db: &SourceDatabase,
    input: &RestoreSampleInput<'_>,
    file_size: u64,
    modified_ns: i64,
) -> Result<(), String> {
    db.upsert_file(input.relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
    db.set_tag(input.relative_path, input.tag)
        .map_err(|err| format!("Failed to sync tag: {err}"))?;
    db.set_looped(input.relative_path, input.looped)
        .map_err(|err| format!("Failed to sync loop metadata: {err}"))?;
    write_playback_age(db, input.relative_path, input.last_played_at)?;
    write_normal_tags(db, input.relative_path, &input.normal_tags)
}

fn write_playback_age(
    db: &SourceDatabase,
    relative_path: &Path,
    last_played_at: Option<i64>,
) -> Result<(), String> {
    if let Some(last_played_at) = last_played_at {
        db.set_last_played_at(relative_path, last_played_at)
    } else {
        db.clear_last_played_at(relative_path)
    }
    .map_err(|err| format!("Failed to sync playback age: {err}"))
}

fn write_normal_tags(
    db: &SourceDatabase,
    relative_path: &Path,
    normal_tags: &[String],
) -> Result<(), String> {
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to sync normal tags: {err}"))?;
    batch
        .replace_tags_for_path(relative_path, normal_tags)
        .map_err(|err| format!("Failed to sync normal tags: {err}"))?;
    batch
        .commit()
        .map_err(|err| format!("Failed to sync normal tags: {err}"))
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create folder {}: {err}", parent.display()))?;
    }
    Ok(())
}

fn open_source_db(source_root: &Path) -> Result<SourceDatabase, String> {
    SourceDatabase::open_for_source_write(source_root)
        .map_err(|err| format!("Database unavailable: {err}"))
}

fn delete_sample_file_if_present(path: &Path) -> Result<(), String> {
    match remove_sample_file_with_retry(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("Failed to delete sample {}: {err}", path.display())),
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
    std::fs::remove_file(path)
}
