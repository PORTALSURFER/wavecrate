use std::path::Path;
use std::time::UNIX_EPOCH;

use super::super::SourceDatabase;
use super::FileOpJournalEntry;
use super::entry::{FileOpKind, FileOpStage};
use super::recovery_io::{
    RecoveryFilesystem, RecoverySourceDatabases, SourceDatabaseRecoveryAccess,
    SystemRecoveryFilesystem,
};
use super::store::FileOpJournalStore;

struct FileOpRecoveryCoordinator<'a, F, D> {
    target_db: &'a SourceDatabase,
    journal: FileOpJournalStore<'a>,
    filesystem: F,
    source_databases: D,
}

/// Summary of reconciliation work performed for pending file ops.
#[derive(Debug, Default)]
pub struct FileOpReconcileSummary {
    /// Total number of journal rows considered, including malformed rows.
    pub total: usize,
    /// Number of rows successfully reconciled and removed.
    pub completed: usize,
    /// Human-readable reconciliation errors.
    pub errors: Vec<String>,
}

/// Reconcile all pending file ops against the filesystem and database.
pub fn reconcile_pending_ops(db: &SourceDatabase) -> Result<FileOpReconcileSummary, String> {
    FileOpRecoveryCoordinator {
        target_db: db,
        journal: FileOpJournalStore::new(db),
        filesystem: SystemRecoveryFilesystem,
        source_databases: SourceDatabaseRecoveryAccess,
    }
    .reconcile()
}

impl<F: RecoveryFilesystem, D: RecoverySourceDatabases> FileOpRecoveryCoordinator<'_, F, D> {
    fn reconcile(&self) -> Result<FileOpReconcileSummary, String> {
        let listed = self.journal.list().map_err(|err| err.to_string())?;
        let mut summary = FileOpReconcileSummary {
            total: listed.entries.len() + listed.malformed.len(),
            completed: 0,
            errors: Vec::new(),
        };
        for malformed in listed.malformed {
            let message = malformed.describe();
            if let Some(id) = malformed.id.as_deref() {
                match self.journal.remove(id) {
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
            match reconcile_entry(
                self.target_db,
                &entry,
                &self.filesystem,
                &self.source_databases,
            ) {
                Ok(()) => {
                    if let Err(err) = self.journal.remove(&entry.id) {
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
}

fn reconcile_entry(
    db: &SourceDatabase,
    entry: &FileOpJournalEntry,
    filesystem: &impl RecoveryFilesystem,
    source_databases: &impl RecoverySourceDatabases,
) -> Result<(), String> {
    let target_root = db.root();
    let target_absolute = target_root.join(&entry.target_relative);
    let staged_absolute = entry
        .staged_relative
        .as_ref()
        .map(|path| target_root.join(path));
    validate_staged_file_identity(entry, staged_absolute.as_deref(), filesystem)?;
    validate_existing_target_identity(
        entry,
        staged_absolute.as_deref(),
        &target_absolute,
        filesystem,
    )?;
    reconcile_staged_file(staged_absolute.as_deref(), &target_absolute, filesystem)?;
    let target_exists = reconcile_target_entry(db, entry, &target_absolute, filesystem)?;
    if entry.kind == FileOpKind::Move {
        reconcile_source_entry(db, entry, target_exists, filesystem, source_databases)?;
    }
    Ok(())
}

/// Finalize one staged file into the target path or clean the stale staged copy.
fn reconcile_staged_file(
    staged_absolute: Option<&Path>,
    target_absolute: &Path,
    filesystem: &impl RecoveryFilesystem,
) -> Result<(), String> {
    let Some(staged_absolute) = staged_absolute else {
        return Ok(());
    };
    if !filesystem.is_file(staged_absolute) {
        return Ok(());
    }
    if !filesystem.is_file(target_absolute) {
        if let Some(parent) = target_absolute.parent() {
            filesystem
                .create_dir_all(parent)
                .map_err(|err| format!("Failed to create target dir: {err}"))?;
        }
        filesystem
            .rename(staged_absolute, target_absolute)
            .map_err(|err| format!("Failed to finalize staged file: {err}"))?;
    } else {
        filesystem
            .remove_file(staged_absolute)
            .map_err(|err| format!("Failed to remove staged file: {err}"))?;
    }
    Ok(())
}

fn validate_staged_file_identity(
    entry: &FileOpJournalEntry,
    staged_absolute: Option<&Path>,
    filesystem: &impl RecoveryFilesystem,
) -> Result<(), String> {
    let Some(staged_absolute) = staged_absolute else {
        return Ok(());
    };
    if !filesystem.is_file(staged_absolute) {
        return Ok(());
    }
    validate_identity_match(
        entry,
        staged_absolute,
        "staged",
        "staged file no longer matches the recorded journal metadata",
        filesystem,
    )
}

fn validate_existing_target_identity(
    entry: &FileOpJournalEntry,
    staged_absolute: Option<&Path>,
    target_absolute: &Path,
    filesystem: &impl RecoveryFilesystem,
) -> Result<(), String> {
    if !filesystem.is_file(target_absolute) {
        return Ok(());
    }
    if entry.file_size.is_none() || entry.modified_ns.is_none() {
        return Err(format!(
            "Deferred file-op recovery for {}: target file {} exists but journaled identity is incomplete; {}",
            entry.id,
            target_absolute.display(),
            staged_copy_resolution_suffix(staged_absolute, filesystem)
        ));
    }
    validate_identity_match(
        entry,
        target_absolute,
        "target",
        &format!(
            "target path was reused before recovery replay{}",
            staged_copy_resolution_suffix(staged_absolute, filesystem)
        ),
        filesystem,
    )
}

fn staged_copy_resolution_suffix(
    staged_absolute: Option<&Path>,
    filesystem: &impl RecoveryFilesystem,
) -> String {
    if staged_absolute.is_some_and(|path| filesystem.is_file(path)) {
        format!(
            "; leaving staged copy at {} intact",
            staged_absolute.expect("checked above").display()
        )
    } else {
        String::from("; no staged copy remains to reconcile safely")
    }
}

fn validate_identity_match(
    entry: &FileOpJournalEntry,
    path: &Path,
    location: &str,
    mismatch_reason: &str,
    filesystem: &impl RecoveryFilesystem,
) -> Result<(), String> {
    let Some(expected_file_size) = entry.file_size else {
        return Ok(());
    };
    let Some(expected_modified_ns) = entry.modified_ns else {
        return Ok(());
    };
    let actual = file_metadata(path, filesystem)?;
    if actual == (expected_file_size, expected_modified_ns) {
        return Ok(());
    }
    Err(format!(
        "Deferred file-op recovery for {}: {} file {} does not match journaled identity (expected {} bytes @ {}, found {} bytes @ {}); {}",
        entry.id,
        location,
        path.display(),
        expected_file_size,
        expected_modified_ns,
        actual.0,
        actual.1,
        mismatch_reason
    ))
}

/// Reconcile one target DB row and return whether the target file exists afterwards.
fn reconcile_target_entry(
    db: &SourceDatabase,
    entry: &FileOpJournalEntry,
    target_absolute: &Path,
    filesystem: &impl RecoveryFilesystem,
) -> Result<bool, String> {
    if filesystem.is_file(target_absolute) {
        let (file_size, modified_ns) = file_metadata(target_absolute, filesystem)?;
        let mut batch = db.write_batch().map_err(|err| err.to_string())?;
        match entry.kind {
            FileOpKind::Copy => batch
                .upsert_file_without_hash(&entry.target_relative, file_size, modified_ns)
                .map_err(|err| err.to_string())?,
            FileOpKind::Move => batch
                .upsert_file(&entry.target_relative, file_size, modified_ns)
                .map_err(|err| err.to_string())?,
        }
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
        if let Some(locked) = entry.locked {
            batch
                .set_locked(&entry.target_relative, locked)
                .map_err(|err| err.to_string())?;
        }
        if let Some(last_played_at) = entry.last_played_at {
            batch
                .set_last_played_at(&entry.target_relative, last_played_at)
                .map_err(|err| err.to_string())?;
        } else {
            batch
                .clear_last_played_at(&entry.target_relative)
                .map_err(|err| err.to_string())?;
        }
        if let Some(last_curated_at) = entry.last_curated_at {
            batch
                .set_last_curated_at(&entry.target_relative, last_curated_at)
                .map_err(|err| err.to_string())?;
        } else {
            batch
                .clear_last_curated_at(&entry.target_relative)
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
    filesystem: &impl RecoveryFilesystem,
    source_databases: &impl RecoverySourceDatabases,
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
    if filesystem.is_file(&source_absolute) && !target_exists {
        return Ok(());
    }
    let source_db = source_databases.open(source_root)?;
    if !filesystem.is_file(&source_absolute) {
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

fn file_metadata(path: &Path, filesystem: &impl RecoveryFilesystem) -> Result<(u64, i64), String> {
    let metadata = filesystem
        .metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "File modified time is before epoch".to_string())?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}
