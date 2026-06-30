use super::{
    DeleteJournalEntry, DeleteJournalStage, DeleteRecoveryAction, DeleteRecoveryEntry,
    RetainedDeleteEntry, SampleSource, recovery_entry, restore_staged_folder, retained,
};
use crate::app::controller::library::source_folders::delete_recovery::path_policy;
use std::path::{Path, PathBuf};

pub(super) struct JournaledRecovery {
    pub(super) report_entry: DeleteRecoveryEntry,
    pub(super) remove_from_journal: bool,
    pub(super) needs_hard_sync: bool,
}

pub(super) struct RetainedRecovery {
    pub(super) retained_entry: RetainedDeleteEntry,
}

pub(super) enum JournaledRecoveryOutcome {
    Completed(JournaledRecovery),
    Retained(RetainedRecovery),
}

pub(super) fn recover_journaled_entry(
    source: &SampleSource,
    staging_root: &Path,
    entry: &DeleteJournalEntry,
) -> Option<JournaledRecoveryOutcome> {
    let original_relative =
        match path_policy::validate_journal_relative(&entry.original_relative, "original_relative")
        {
            Ok(path) => path,
            Err(err) => {
                return Some(invalid_journal_entry(
                    source,
                    PathBuf::from(&entry.original_relative),
                    err,
                ));
            }
        };
    let staged_relative =
        match path_policy::validate_journal_relative(&entry.staged_relative, "staged_relative") {
            Ok(path) => path,
            Err(err) => return Some(invalid_journal_entry(source, original_relative, err)),
        };
    let staged = staging_root.join(&staged_relative);
    let original = source.root.join(&original_relative);
    let (action, outcome) = match entry.stage {
        DeleteJournalStage::Deleted => {
            return retained::recover_retained_delete(
                source,
                &original_relative,
                &staged_relative,
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
                &staged_relative,
                &staged,
                &original,
                entry,
            ));
        }
        DeleteJournalStage::Intent | DeleteJournalStage::Staged => {
            let outcome = if !path_policy::path_exists_no_follow(&staged).unwrap_or(false)
                && path_policy::path_exists_no_follow(&original).unwrap_or(false)
            {
                path_policy::ensure_existing_path_under(
                    &source.root,
                    &original,
                    "Restored delete folder",
                )
                .map(|_| Some("Already restored".into()))
            } else {
                restore_staged_folder(&staged, &original, staging_root, &source.root)
            };
            (DeleteRecoveryAction::Restore, outcome)
        }
    };
    let remove_from_journal = outcome.is_ok();
    Some(JournaledRecoveryOutcome::Completed(JournaledRecovery {
        report_entry: recovery_entry(source, original_relative, action, outcome),
        remove_from_journal,
        needs_hard_sync: false,
    }))
}

fn invalid_journal_entry(
    source: &SampleSource,
    original_relative: PathBuf,
    err: String,
) -> JournaledRecoveryOutcome {
    JournaledRecoveryOutcome::Completed(JournaledRecovery {
        report_entry: recovery_entry(
            source,
            original_relative,
            DeleteRecoveryAction::Restore,
            Err(format!("Invalid delete journal entry: {err}")),
        ),
        remove_from_journal: false,
        needs_hard_sync: false,
    })
}
