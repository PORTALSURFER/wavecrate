//! Startup recovery for staged folder deletes.
//!
//! Recovery walks each source-local staging directory and applies the journal contract:
//! - `Intent` or `Staged` means the original folder should exist after recovery
//! - `Deleted` means the staged folder should remain retained as app-owned trash
//! - `RestorePendingDb` means an explicit retained restore must finish merge/DB replay
//! - staged folders that exist without journal entries are conservatively restored
use super::DELETE_STAGING_DIR;
use super::DeleteStagingInfo;
use super::journal::{
    DeleteJournal, DeleteJournalEntry, DeleteJournalStage, load_journal, remove_entry,
};
use super::restore_merge::restore_retained_folder_with_merge_with_stamp;
use super::retained_restore_reconcile::{
    apply_retained_restore_db_entries, infer_retained_restore_merge_report,
    snapshot_existing_restore_entries,
};
use crate::sample_sources::{SampleSource, SourceId, WavEntry};
use std::fs;
use std::path::{Path, PathBuf};

mod retained;
mod scan;

const RESTORE_SUFFIX: &str = ".restored";
const DELETE_JOURNAL_FILE: &str = "delete_journal.json";
/// Recovery action taken for a staged delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteRecoveryAction {
    /// Move the staged folder back into the source tree.
    Restore,
    /// Permanently delete the staged folder.
    Finalize,
}
/// Outcome for a recovery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteRecoveryStatus {
    /// Recovery action succeeded.
    Completed,
    /// Recovery action failed and needs attention.
    Failed,
}

/// Per-folder recovery result for UI reporting.
#[derive(Debug, Clone)]
pub(crate) struct DeleteRecoveryEntry {
    /// Source identifier for the staged folder.
    pub(crate) source_id: SourceId,
    /// Source root for display and follow-up refreshes.
    pub(crate) source_root: PathBuf,
    /// Original relative path within the source.
    pub(crate) original_relative: PathBuf,
    /// Action taken during recovery.
    pub(crate) action: DeleteRecoveryAction,
    /// Outcome of the action.
    pub(crate) status: DeleteRecoveryStatus,
    /// Optional extra detail for the UI.
    pub(crate) detail: Option<String>,
}

/// Retained staged delete that remains recoverable after startup reconciliation.
#[derive(Debug, Clone)]
pub(crate) struct RetainedDeleteEntry {
    /// Stable journal identifier for the retained delete.
    pub(crate) id: String,
    /// Source identifier that owns the retained delete.
    pub(crate) source_id: SourceId,
    /// Source root for restore or purge follow-up work.
    pub(crate) source_root: PathBuf,
    /// Original relative folder path within the source.
    pub(crate) original_relative: PathBuf,
    /// Relative staged path inside `.sempal_delete_staging`.
    pub(crate) staged_relative: PathBuf,
    /// Deleted wav metadata snapshot used to reconstruct DB state after restart.
    pub(crate) deleted_entries: Vec<WavEntry>,
}

/// Summary of staged delete recovery across all sources.
#[derive(Debug, Default)]
pub(crate) struct DeleteRecoveryReport {
    /// Per-folder recovery outcomes.
    pub(crate) entries: Vec<DeleteRecoveryEntry>,
    /// Retained deletes that remain available for explicit restore or purge.
    pub(crate) retained_entries: Vec<RetainedDeleteEntry>,
    /// Non-fatal errors encountered during recovery.
    pub(crate) errors: Vec<String>,
}

/// Recover staged deletes for the provided sources.
pub(crate) fn recover_staged_deletes(sources: &[SampleSource]) -> DeleteRecoveryReport {
    let mut report = DeleteRecoveryReport::default();
    for source in sources {
        recover_source(source, &mut report);
    }
    report
}

fn recover_source(source: &SampleSource, report: &mut DeleteRecoveryReport) {
    if !source.root.is_dir() {
        return;
    }
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    if !staging_root.is_dir() {
        return;
    }
    let journal = match load_journal(&staging_root) {
        Ok(journal) => journal,
        Err(err) => {
            report.errors.push(format!(
                "Failed to read delete journal for {}: {err}",
                source.root.display()
            ));
            DeleteJournal::default()
        }
    };
    let journaled_roots = journaled_staged_roots(&journal);
    recover_journaled_entries(source, &staging_root, &journal, report);
    recover_unjournaled_entries(source, &staging_root, &journaled_roots, report);
    super::cleanup_staging_root(&staging_root);
}

fn recover_journaled_entries(
    source: &SampleSource,
    staging_root: &Path,
    journal: &DeleteJournal,
    report: &mut DeleteRecoveryReport,
) {
    for entry in journal.entries.clone() {
        match recover_journaled_entry(source, staging_root, &entry) {
            Some(JournaledRecoveryOutcome::Completed(result)) => {
                report.entries.push(result.report_entry);
                if result.remove_from_journal
                    && let Err(err) = remove_entry(staging_root, &entry.id)
                {
                    report
                        .errors
                        .push(format!("Failed to update delete journal: {err}"));
                }
            }
            Some(JournaledRecoveryOutcome::Retained(result)) => {
                report.retained_entries.push(result.retained_entry);
            }
            None => {}
        }
    }
}

fn recover_unjournaled_entries(
    source: &SampleSource,
    staging_root: &Path,
    journaled_roots: &[PathBuf],
    report: &mut DeleteRecoveryReport,
) {
    for relative in find_unjournaled_staged_roots(staging_root, &source.root, journaled_roots) {
        let staged = staging_root.join(&relative);
        let original = source.root.join(&relative);
        report.entries.push(recovery_entry(
            source,
            relative,
            DeleteRecoveryAction::Restore,
            restore_staged_folder(&staged, &original),
        ));
    }
}

struct JournaledRecovery {
    report_entry: DeleteRecoveryEntry,
    remove_from_journal: bool,
}

struct RetainedRecovery {
    retained_entry: RetainedDeleteEntry,
}

enum JournaledRecoveryOutcome {
    Completed(JournaledRecovery),
    Retained(RetainedRecovery),
}

fn recover_journaled_entry(
    source: &SampleSource,
    staging_root: &Path,
    entry: &DeleteJournalEntry,
) -> Option<JournaledRecoveryOutcome> {
    let original_relative = PathBuf::from(entry.original_relative.clone());
    let staged = staging_root.join(&entry.staged_relative);
    let original = source.root.join(&original_relative);
    let (action, outcome) = match entry.stage {
        DeleteJournalStage::Deleted => {
            return retained::recover_retained_delete(
                source,
                &original_relative,
                &staged,
                &original,
                entry,
            );
        }
        DeleteJournalStage::RestorePendingDb => {
            return Some(retained::recover_pending_retained_restore(
                source,
                staging_root,
                &original_relative,
                &staged,
                &original,
                entry,
            ));
        }
        DeleteJournalStage::Intent | DeleteJournalStage::Staged => {
            let outcome = if !staged.exists() && original.exists() {
                Ok(Some("Already restored".into()))
            } else {
                restore_staged_folder(&staged, &original)
            };
            (DeleteRecoveryAction::Restore, outcome)
        }
    };
    let remove_from_journal = outcome.is_ok();
    Some(JournaledRecoveryOutcome::Completed(JournaledRecovery {
        report_entry: recovery_entry(source, original_relative, action, outcome),
        remove_from_journal,
    }))
}

fn recovery_entry(
    source: &SampleSource,
    original_relative: PathBuf,
    action: DeleteRecoveryAction,
    outcome: Result<Option<String>, String>,
) -> DeleteRecoveryEntry {
    let (status, detail) = match outcome {
        Ok(detail) => (DeleteRecoveryStatus::Completed, detail),
        Err(err) => (DeleteRecoveryStatus::Failed, Some(err)),
    };
    DeleteRecoveryEntry {
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        original_relative,
        action,
        status,
        detail,
    }
}

fn restore_staged_folder(staged: &Path, original: &Path) -> Result<Option<String>, String> {
    if !staged.exists() {
        return Err("Staged folder missing".into());
    }
    let (target, detail) = unique_restore_path(original);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("Failed to restore folder: {err}"))?;
    }
    fs::rename(staged, &target).map_err(|err| format!("Failed to restore folder: {err}"))?;
    Ok(detail)
}

fn unique_restore_path(original: &Path) -> (PathBuf, Option<String>) {
    if !original.exists() {
        return (original.to_path_buf(), None);
    }
    let parent = original.parent().unwrap_or_else(|| Path::new(""));
    let name = original
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("folder");
    for idx in 1..=1000 {
        let candidate = parent.join(format!("{name}{RESTORE_SUFFIX}-{idx}"));
        if !candidate.exists() {
            return (
                candidate.clone(),
                Some(format!("Restored as {}", candidate.display())),
            );
        }
    }
    let fallback = parent.join(format!("{name}{RESTORE_SUFFIX}-{}", uuid::Uuid::new_v4()));
    (
        fallback.clone(),
        Some(format!("Restored as {}", fallback.display())),
    )
}

fn journaled_staged_roots(journal: &DeleteJournal) -> Vec<PathBuf> {
    scan::journaled_staged_roots(journal)
}

fn find_unjournaled_staged_roots(
    staging_root: &Path,
    source_root: &Path,
    journaled_roots: &[PathBuf],
) -> Vec<PathBuf> {
    scan::find_unjournaled_staged_roots(staging_root, source_root, journaled_roots)
}

#[cfg(test)]
mod tests;
