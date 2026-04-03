use super::*;

fn completed_recovery(
    source: &SampleSource,
    original_relative: PathBuf,
    action: DeleteRecoveryAction,
    outcome: Result<Option<String>, String>,
) -> JournaledRecoveryOutcome {
    let remove_from_journal = outcome.is_ok();
    JournaledRecoveryOutcome::Completed(JournaledRecovery {
        report_entry: recovery_entry(
            source,
            original_relative,
            action,
            outcome,
        ),
        remove_from_journal,
    })
}

pub(super) fn recover_retained_delete(
    source: &SampleSource,
    original_relative: &Path,
    staged: &Path,
    original: &Path,
    entry: &DeleteJournalEntry,
) -> Option<JournaledRecoveryOutcome> {
    if staged.exists() && !original.exists() {
        return Some(JournaledRecoveryOutcome::Retained(RetainedRecovery {
            retained_entry: RetainedDeleteEntry {
                id: entry.id.clone(),
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                original_relative: original_relative.to_path_buf(),
                staged_relative: PathBuf::from(entry.staged_relative.clone()),
                deleted_entries: entry.deleted_entries.clone(),
            },
        }));
    }
    if !staged.exists() && original.exists() {
        return Some(completed_recovery(
            source,
            original_relative.to_path_buf(),
            DeleteRecoveryAction::Restore,
            Ok(Some("Already restored".into())),
        ));
    }
    if !staged.exists() && !original.exists() {
        return Some(completed_recovery(
            source,
            original_relative.to_path_buf(),
            DeleteRecoveryAction::Finalize,
            Ok(Some("Already purged".into())),
        ));
    }
    Some(completed_recovery(
        source,
        original_relative.to_path_buf(),
        DeleteRecoveryAction::Finalize,
        Err(format!(
            "Retained delete state is inconsistent (original exists: {}, staged exists: {})",
            original.exists(),
            staged.exists()
        )),
    ))
}

pub(super) fn recover_pending_retained_restore(
    source: &SampleSource,
    staging_root: &Path,
    original_relative: &Path,
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
        if staged.exists() {
            let staged_info = DeleteStagingInfo {
                id: entry.id.clone(),
                original_relative: original_relative.to_path_buf(),
                staged_relative: PathBuf::from(entry.staged_relative.clone()),
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
    )
}
