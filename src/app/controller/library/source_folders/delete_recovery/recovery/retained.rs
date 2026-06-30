use super::*;
use crate::app::controller::library::source_folders::delete_recovery::path_policy;

fn completed_recovery(
    source: &SampleSource,
    original_relative: PathBuf,
    action: DeleteRecoveryAction,
    outcome: Result<Option<String>, String>,
    needs_hard_sync: bool,
) -> JournaledRecoveryOutcome {
    let remove_from_journal = outcome.is_ok();
    JournaledRecoveryOutcome::Completed(JournaledRecovery {
        report_entry: recovery_entry(source, original_relative, action, outcome),
        remove_from_journal,
        needs_hard_sync: remove_from_journal && needs_hard_sync,
    })
}

pub(super) fn recover_retained_delete(
    source: &SampleSource,
    original_relative: &Path,
    staged_relative: &Path,
    staged: &Path,
    original: &Path,
    entry: &DeleteJournalEntry,
) -> Option<JournaledRecoveryOutcome> {
    match path_policy::path_exists_no_follow(staged) {
        Ok(true) => {
            if let Err(err) = path_policy::ensure_existing_dir_under(
                &source.root.join(DELETE_STAGING_DIR),
                staged,
                "Retained staged folder",
            ) {
                return Some(completed_recovery(
                    source,
                    original_relative.to_path_buf(),
                    DeleteRecoveryAction::Restore,
                    Err(err),
                    false,
                ));
            }
            return Some(JournaledRecoveryOutcome::Retained(RetainedRecovery {
                retained_entry: RetainedDeleteEntry {
                    id: entry.id.clone(),
                    source_id: source.id.clone(),
                    source_root: source.root.clone(),
                    original_relative: original_relative.to_path_buf(),
                    staged_relative: staged_relative.to_path_buf(),
                    deleted_entries: entry.deleted_entries.clone(),
                },
            }));
        }
        Ok(false) => {}
        Err(err) => {
            return Some(completed_recovery(
                source,
                original_relative.to_path_buf(),
                DeleteRecoveryAction::Restore,
                Err(err),
                false,
            ));
        }
    }
    match path_policy::path_exists_no_follow(original) {
        Ok(true) => {
            return Some(completed_recovery(
                source,
                original_relative.to_path_buf(),
                DeleteRecoveryAction::Restore,
                path_policy::ensure_existing_path_under(
                    &source.root,
                    original,
                    "Restored retained folder",
                )
                .map(|_| Some("Already restored".into())),
                false,
            ));
        }
        Ok(false) => {}
        Err(err) => {
            return Some(completed_recovery(
                source,
                original_relative.to_path_buf(),
                DeleteRecoveryAction::Restore,
                Err(err),
                false,
            ));
        }
    }
    Some(completed_recovery(
        source,
        original_relative.to_path_buf(),
        DeleteRecoveryAction::Finalize,
        Ok(Some("Already purged".into())),
        false,
    ))
}

pub(super) fn recover_pending_retained_restore(
    source: &SampleSource,
    staging_root: &Path,
    original_relative: &Path,
    staged_relative: &Path,
    staged: &Path,
    original: &Path,
    entry: &DeleteJournalEntry,
) -> JournaledRecoveryOutcome {
    let outcome = (|| -> Result<Option<String>, String> {
        let stamp = entry
            .restore_stamp
            .as_deref()
            .ok_or_else(|| "Retained restore stamp missing".to_string())?;
        let existing_entries = snapshot_existing_restore_entries(source, &entry.deleted_entries)?;
        if path_policy::path_exists_no_follow(staged)? {
            path_policy::ensure_existing_dir_under(
                staging_root,
                staged,
                "Pending retained staged folder",
            )?;
            path_policy::ensure_creatable_path_under(
                &source.root,
                original,
                "Pending retained restore destination",
            )?;
            let staged_info = DeleteStagingInfo {
                id: entry.id.clone(),
                original_relative: original_relative.to_path_buf(),
                staged_relative: staged_relative.to_path_buf(),
                staged_absolute: staged.to_path_buf(),
            };
            restore_retained_folder_with_merge_with_stamp(
                &staged_info,
                &source.root,
                original,
                staging_root,
                stamp,
            )?;
        }
        let merge = infer_retained_restore_merge_report(
            &source.root,
            &entry.deleted_entries,
            &existing_entries,
            stamp,
        )?;
        apply_retained_restore_db_entries(
            source,
            &entry.deleted_entries,
            &existing_entries,
            &merge,
        )?;
        Ok(Some("Completed retained restore after restart".into()))
    })();
    completed_recovery(
        source,
        original_relative.to_path_buf(),
        DeleteRecoveryAction::Restore,
        outcome,
        entry.deleted_entries.is_empty(),
    )
}
