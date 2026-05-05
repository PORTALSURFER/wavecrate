use super::load_journal;
use super::remove_entry;
use super::store::{insert_entry, update_entry, update_journal_stage};
use super::{DeleteJournal, DeleteJournalEntry, DeleteJournalStage, DeleteStagingInfo};
use crate::sample_sources::WavEntry;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Stage a folder for deletion and record it in the journal.
pub(crate) fn stage_folder_for_delete(
    absolute: &Path,
    staging_root: &Path,
    relative: &Path,
    deleted_entries: &[WavEntry],
) -> Result<DeleteStagingInfo, String> {
    let journal = load_journal(staging_root)?;
    let staged_relative = unique_staging_relative(staging_root, &journal, relative);
    let staged_absolute = staging_root.join(&staged_relative);
    ensure_staging_parent(&staged_absolute, staging_root)?;
    let id = new_delete_op_id();
    insert_entry(
        staging_root,
        DeleteJournalEntry {
            id: id.clone(),
            original_relative: relative.to_string_lossy().to_string(),
            staged_relative: staged_relative.to_string_lossy().to_string(),
            deleted_entries: deleted_entries.to_vec(),
            stage: DeleteJournalStage::Intent,
            restore_stamp: None,
            created_at: now_epoch_seconds()?,
        },
    )?;
    if let Err(err) = fs::rename(absolute, &staged_absolute) {
        let _ = remove_entry(staging_root, &id);
        return Err(format!("Failed to stage folder delete: {err}"));
    }
    if let Err(err) = update_journal_stage(staging_root, &id, DeleteJournalStage::Staged) {
        let _ = fs::rename(&staged_absolute, absolute);
        let _ = remove_entry(staging_root, &id);
        return Err(format!("Failed to record delete staging: {err}"));
    }
    Ok(DeleteStagingInfo {
        id,
        original_relative: relative.to_path_buf(),
        staged_relative,
        staged_absolute,
    })
}

/// Mark a staged delete as a retained app-owned delete after DB updates succeed.
pub(crate) fn mark_delete_retained(staging_root: &Path, id: &str) -> Result<(), String> {
    update_journal_stage(staging_root, id, DeleteJournalStage::Deleted)
}

/// Mark a retained delete as actively restoring so restart recovery can finish DB replay.
pub(crate) fn mark_delete_restore_pending_db(
    staging_root: &Path,
    id: &str,
    stamp: &str,
) -> Result<(), String> {
    update_entry(staging_root, id, |entry| {
        entry.stage = DeleteJournalStage::RestorePendingDb;
        entry.restore_stamp = Some(stamp.to_string());
    })
}

/// Remove a journal entry after a staged delete is resolved.
pub(crate) fn remove_delete_entry(staging_root: &Path, id: &str) -> Result<(), String> {
    remove_entry(staging_root, id)
}

/// Permanently remove a retained staged folder and clear its journal entry.
pub(crate) fn purge_deleted_folder(
    info: &DeleteStagingInfo,
    staging_root: &Path,
) -> Result<(), String> {
    if info.staged_absolute.exists() {
        fs::remove_dir_all(&info.staged_absolute).map_err(|err| {
            format!(
                "Failed to purge deleted folder {}: {err}",
                info.original_relative.display()
            )
        })?;
    }
    remove_entry(staging_root, &info.id)?;
    cleanup_staging_root(staging_root);
    Ok(())
}

/// Restore a retained staged folder into the source tree and remove its journal entry.
pub(crate) fn restore_deleted_folder(
    info: &DeleteStagingInfo,
    absolute: &Path,
    staging_root: &Path,
) -> Result<(), String> {
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to prepare restore destination {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::rename(&info.staged_absolute, absolute).map_err(|err| {
        format!(
            "Failed to restore deleted folder {}: {err}",
            info.original_relative.display()
        )
    })?;
    remove_entry(staging_root, &info.id)?;
    cleanup_staging_root(staging_root);
    Ok(())
}

/// Re-stage a restored folder back into the retained delete area for redo.
pub(crate) fn restage_deleted_folder(
    absolute: &Path,
    staging_root: &Path,
    info: &DeleteStagingInfo,
    deleted_entries: &[WavEntry],
) -> Result<(), String> {
    let entry = DeleteJournalEntry {
        id: info.id.clone(),
        original_relative: info.original_relative.to_string_lossy().to_string(),
        staged_relative: info.staged_relative.to_string_lossy().to_string(),
        deleted_entries: deleted_entries.to_vec(),
        stage: DeleteJournalStage::Intent,
        restore_stamp: None,
        created_at: now_epoch_seconds()?,
    };
    ensure_staging_parent(&info.staged_absolute, staging_root)?;
    insert_entry(staging_root, entry)?;
    if let Err(err) = fs::rename(absolute, &info.staged_absolute) {
        let _ = remove_entry(staging_root, &info.id);
        return Err(format!(
            "Failed to re-stage deleted folder {}: {err}",
            info.original_relative.display()
        ));
    }
    if let Err(err) = update_journal_stage(staging_root, &info.id, DeleteJournalStage::Staged) {
        let _ = fs::rename(&info.staged_absolute, absolute);
        let _ = remove_entry(staging_root, &info.id);
        return Err(format!(
            "Failed to record restored folder delete staging for {}: {err}",
            info.original_relative.display()
        ));
    }
    Ok(())
}

/// Roll back a staged folder delete, restoring the folder and journal state.
pub(crate) fn rollback_staged_folder(
    info: &DeleteStagingInfo,
    absolute: &Path,
    staging_root: &Path,
    err: &str,
) -> Result<(), String> {
    let rollback_context = format!(
        " (original: {}, staged: {})",
        info.original_relative.display(),
        info.staged_relative.display()
    );
    if let Err(restore_err) = fs::rename(&info.staged_absolute, absolute) {
        return Err(format!(
            "{err}{rollback_context} (also failed to restore folder: {restore_err})"
        ));
    }
    let _ = remove_entry(staging_root, &info.id);
    cleanup_staging_root(staging_root);
    Err(format!("{err}{rollback_context}"))
}

/// Remove the staging root if it is now empty.
pub(crate) fn cleanup_staging_root(staging_root: &Path) {
    if let Ok(mut entries) = fs::read_dir(staging_root)
        && entries.next().is_none()
    {
        let _ = fs::remove_dir(staging_root);
    }
}

fn unique_staging_relative(
    staging_root: &Path,
    journal: &DeleteJournal,
    relative: &Path,
) -> PathBuf {
    let mut candidate = relative.to_path_buf();
    if staging_relative_is_available(staging_root, journal, &candidate) {
        return candidate;
    }
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let name = relative
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("folder");
    for idx in 1..=1000 {
        let mut alternative = PathBuf::from(parent);
        alternative.push(format!("{name}.staged-{idx}"));
        candidate = alternative;
        if staging_relative_is_available(staging_root, journal, &candidate) {
            return candidate;
        }
    }
    candidate
}

fn staging_relative_is_available(
    staging_root: &Path,
    journal: &DeleteJournal,
    candidate: &Path,
) -> bool {
    !staging_root.join(candidate).exists()
        && !journal
            .entries
            .iter()
            .any(|entry| Path::new(&entry.staged_relative) == candidate)
}

fn ensure_staging_parent(staged: &Path, staging_root: &Path) -> Result<(), String> {
    if let Some(parent) = staged.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to prepare folder delete staging: {err}"))?;
        mark_staging_root_hidden(staging_root);
    }
    Ok(())
}

fn mark_staging_root_hidden(staging_root: &Path) {
    #[cfg(target_os = "windows")]
    {
        use super::super::DELETE_STAGING_DIR;
        use windows::{
            Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, SetFileAttributesW},
            core::PCWSTR,
        };

        if staging_root
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == DELETE_STAGING_DIR)
        {
            let mut wide: Vec<u16> =
                <std::ffi::OsStr as std::os::windows::ffi::OsStrExt>::encode_wide(
                    staging_root.as_os_str(),
                )
                .collect();
            wide.push(0);
            let _ = unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_ATTRIBUTE_HIDDEN) };
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = staging_root;
}

fn new_delete_op_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn now_epoch_seconds() -> Result<i64, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("System time error: {err}"))?;
    Ok(now.as_secs() as i64)
}
