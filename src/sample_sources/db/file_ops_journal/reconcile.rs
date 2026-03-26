use std::path::Path;
use std::time::UNIX_EPOCH;

use super::super::SourceDatabase;
use super::FileOpJournalEntry;
use super::entry::{FileOpKind, FileOpStage};
use super::store::{list_entries, remove_entry};

/// Summary of reconciliation work performed for pending file ops.
#[derive(Debug, Default)]
pub(crate) struct FileOpReconcileSummary {
    pub(crate) total: usize,
    pub(crate) completed: usize,
    pub(crate) errors: Vec<String>,
}

/// Reconcile all pending file ops against the filesystem and database.
pub(crate) fn reconcile_pending_ops(db: &SourceDatabase) -> Result<FileOpReconcileSummary, String> {
    let listed = list_entries(db).map_err(|err| err.to_string())?;
    let mut summary = FileOpReconcileSummary {
        total: listed.entries.len() + listed.malformed.len(),
        completed: 0,
        errors: Vec::new(),
    };
    for malformed in listed.malformed {
        let message = malformed.describe();
        if let Some(id) = malformed.id.as_deref() {
            match remove_entry(db, id) {
                Ok(()) => summary
                    .errors
                    .push(format!("{message}; dropped malformed journal row")),
                Err(err) => summary.errors.push(format!(
                    "{message}; failed to drop malformed row {id}: {err}"
                )),
            }
        } else {
            summary.errors.push(message);
        }
    }
    for entry in listed.entries {
        match reconcile_entry(db, &entry) {
            Ok(()) => {
                if let Err(err) = remove_entry(db, &entry.id) {
                    summary.errors.push(format!(
                        "Failed to remove journal entry {}: {err}",
                        entry.id
                    ));
                } else {
                    summary.completed += 1;
                }
            }
            Err(err) => summary.errors.push(err),
        }
    }
    Ok(summary)
}

fn reconcile_entry(db: &SourceDatabase, entry: &FileOpJournalEntry) -> Result<(), String> {
    let target_root = db.root();
    let target_absolute = target_root.join(&entry.target_relative);
    let staged_absolute = entry
        .staged_relative
        .as_ref()
        .map(|path| target_root.join(path));
    reconcile_staged_file(staged_absolute.as_deref(), &target_absolute)?;
    let target_exists = reconcile_target_entry(db, entry, &target_absolute)?;
    if entry.kind == FileOpKind::Move {
        reconcile_source_entry(db, entry, target_exists)?;
    }
    Ok(())
}

/// Finalize one staged file into the target path or clean the stale staged copy.
fn reconcile_staged_file(
    staged_absolute: Option<&Path>,
    target_absolute: &Path,
) -> Result<(), String> {
    let Some(staged_absolute) = staged_absolute else {
        return Ok(());
    };
    if !staged_absolute.is_file() {
        return Ok(());
    }
    if !target_absolute.is_file() {
        if let Some(parent) = target_absolute.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create target dir: {err}"))?;
        }
        std::fs::rename(staged_absolute, target_absolute)
            .map_err(|err| format!("Failed to finalize staged file: {err}"))?;
    } else {
        std::fs::remove_file(staged_absolute)
            .map_err(|err| format!("Failed to remove staged file: {err}"))?;
    }
    Ok(())
}

/// Reconcile one target DB row and return whether the target file exists afterwards.
fn reconcile_target_entry(
    db: &SourceDatabase,
    entry: &FileOpJournalEntry,
    target_absolute: &Path,
) -> Result<bool, String> {
    if target_absolute.is_file() {
        let (file_size, modified_ns) = file_metadata(target_absolute)?;
        let mut batch = db.write_batch().map_err(|err| err.to_string())?;
        batch
            .upsert_file(&entry.target_relative, file_size, modified_ns)
            .map_err(|err| err.to_string())?;
        if let Some(tag) = entry.tag {
            batch
                .set_tag(&entry.target_relative, tag)
                .map_err(|err| err.to_string())?;
        }
        if let Some(looped) = entry.looped {
            batch
                .set_looped(&entry.target_relative, looped)
                .map_err(|err| err.to_string())?;
        }
        if let Some(last_played_at) = entry.last_played_at {
            batch
                .set_last_played_at(&entry.target_relative, last_played_at)
                .map_err(|err| err.to_string())?;
        }
        batch.commit().map_err(|err| err.to_string())?;
        Ok(true)
    } else {
        db.remove_file(&entry.target_relative)
            .map_err(|err| format!("Failed to drop target DB row: {err}"))?;
        Ok(false)
    }
}

fn reconcile_source_entry(
    target_db: &SourceDatabase,
    entry: &FileOpJournalEntry,
    target_exists: bool,
) -> Result<(), String> {
    let Some(source_root) = entry.source_root.as_ref() else {
        return Ok(());
    };
    let Some(source_relative) = entry.source_relative.as_ref() else {
        return Ok(());
    };
    if !source_root.is_dir() {
        if should_defer_source_cleanup(entry, target_exists) {
            return Err(format!(
                "Deferred move recovery for {} until source root is available: {}",
                entry.id,
                source_root.display()
            ));
        }
        return Ok(());
    }
    let source_absolute = source_root.join(source_relative);
    if source_absolute.is_file() && !target_exists {
        return Ok(());
    }
    let source_db = SourceDatabase::open(source_root)
        .map_err(|err| format!("Failed to open source DB for recovery: {err}"))?;
    if !source_absolute.is_file() {
        source_db
            .remove_file(source_relative)
            .map_err(|err| format!("Failed to drop source DB row: {err}"))?;
    } else if target_exists {
        tracing::warn!(
            "Move recovery left duplicate file at {} -> {}",
            source_absolute.display(),
            target_db.root().display()
        );
    }
    Ok(())
}

fn should_defer_source_cleanup(entry: &FileOpJournalEntry, target_exists: bool) -> bool {
    target_exists && entry.stage != FileOpStage::SourceDb
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "File modified time is before epoch".to_string())?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}
