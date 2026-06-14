use super::{
    DeleteJournalEntry, DeleteJournalStage, DeleteRecoveryAction, DeleteRecoveryEntry,
    RetainedDeleteEntry, SampleSource, recovery_entry, restore_staged_folder, retained,
};
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
        needs_hard_sync: false,
    }))
}
