//! Journal-backed staging helpers for folder-delete crash recovery.
//!
//! The on-disk journal records the delete lifecycle as
//! `Intent -> Staged -> Deleted -> RestorePendingDb`.
//! Recovery uses that contract to decide whether a folder should be restored back into the
//! source tree after a crash or retained inside the app-owned trash area.

use crate::sample_sources::WavEntry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DELETE_JOURNAL_FILE: &str = "delete_journal.json";

/// Journal stage for a staged folder delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeleteJournalStage {
    /// Intent was recorded before the filesystem rename completed.
    Intent,
    /// Folder data has been moved into the staging area.
    Staged,
    /// Database state was committed, so the staged folder now represents an app-owned delete.
    #[serde(alias = "db_committed")]
    Deleted,
    /// Filesystem restore started and DB metadata replay still needs durable completion.
    RestorePendingDb,
}

/// Metadata for a folder staged for deletion.
#[derive(Debug, Clone)]
pub(crate) struct DeleteStagingInfo {
    /// Unique journal identifier for this staged delete.
    pub(crate) id: String,
    /// Relative path of the original folder within the source.
    pub(crate) original_relative: PathBuf,
    /// Relative path inside the staging root.
    pub(crate) staged_relative: PathBuf,
    /// Absolute staged path on disk.
    pub(crate) staged_absolute: PathBuf,
}

/// Persistent journal entry for a staged folder delete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DeleteJournalEntry {
    pub(super) id: String,
    pub(super) original_relative: String,
    pub(super) staged_relative: String,
    #[serde(default)]
    pub(super) deleted_entries: Vec<WavEntry>,
    pub(super) stage: DeleteJournalStage,
    #[serde(default)]
    pub(super) restore_stamp: Option<String>,
    pub(super) created_at: i64,
}

/// Journal container stored on disk for one source root.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct DeleteJournal {
    pub(super) entries: Vec<DeleteJournalEntry>,
}

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static FAIL_SAVE_BEFORE_REPLACE: AtomicBool = AtomicBool::new(false);

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

pub(super) fn load_journal(staging_root: &Path) -> Result<DeleteJournal, String> {
    let path = journal_path(staging_root);
    if !path.exists() {
        return Ok(DeleteJournal::default());
    }
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read delete journal: {err}"))?;
    serde_json::from_slice(&bytes).map_err(|err| format!("Failed to parse delete journal: {err}"))
}

pub(super) fn remove_entry(staging_root: &Path, id: &str) -> Result<(), String> {
    let mut journal = load_journal(staging_root)?;
    let before = journal.entries.len();
    journal.entries.retain(|entry| entry.id != id);
    if journal.entries.len() == before {
        return Err("Delete journal entry missing".into());
    }
    save_journal(staging_root, &journal)
}

#[cfg(test)]
pub(super) fn update_entry_stage(
    staging_root: &Path,
    id: &str,
    stage: DeleteJournalStage,
) -> Result<(), String> {
    update_journal_stage(staging_root, id, stage)
}

fn update_journal_stage(
    staging_root: &Path,
    id: &str,
    stage: DeleteJournalStage,
) -> Result<(), String> {
    update_entry(staging_root, id, |entry| {
        entry.stage = stage;
        if !matches!(stage, DeleteJournalStage::RestorePendingDb) {
            entry.restore_stamp = None;
        }
    })
}

fn insert_entry(staging_root: &Path, entry: DeleteJournalEntry) -> Result<(), String> {
    let mut journal = load_journal(staging_root)?;
    if journal
        .entries
        .iter()
        .any(|existing| existing.id == entry.id)
    {
        return Err("Delete journal entry already exists".into());
    }
    journal.entries.push(entry);
    save_journal(staging_root, &journal)
}

fn update_entry(
    staging_root: &Path,
    id: &str,
    mutate: impl FnOnce(&mut DeleteJournalEntry),
) -> Result<(), String> {
    let mut journal = load_journal(staging_root)?;
    let entry = journal
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "Delete journal entry missing".to_string())?;
    mutate(entry);
    save_journal(staging_root, &journal)
}

fn save_journal(staging_root: &Path, journal: &DeleteJournal) -> Result<(), String> {
    fs::create_dir_all(staging_root)
        .map_err(|err| format!("Failed to prepare delete journal: {err}"))?;
    let path = journal_path(staging_root);
    if journal.entries.is_empty() {
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|err| format!("Failed to clear delete journal: {err}"))?;
        }
        cleanup_staging_root(staging_root);
        return Ok(());
    }
    let bytes = serde_json::to_vec_pretty(journal)
        .map_err(|err| format!("Failed to serialize delete journal: {err}"))?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, bytes).map_err(|err| format!("Failed to write delete journal: {err}"))?;
    fail_save_before_replace()?;
    replace_journal_file(&tmp_path, &path)?;
    Ok(())
}

fn replace_journal_file(tmp_path: &Path, path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::{
            Win32::Storage::FileSystem::{
                MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
            },
            core::PCWSTR,
        };

        let from = wide_path(tmp_path);
        let to = wide_path(path);
        let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
        unsafe { MoveFileExW(PCWSTR(from.as_ptr()), PCWSTR(to.as_ptr()), flags) }
            .map_err(|err| format!("Failed to save delete journal: {err}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(tmp_path, path).map_err(|err| format!("Failed to save delete journal: {err}"))
    }
}

#[cfg(target_os = "windows")]
fn wide_path(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> =
        <std::ffi::OsStr as std::os::windows::ffi::OsStrExt>::encode_wide(path.as_os_str())
            .collect();
    wide.push(0);
    wide
}

#[cfg(not(test))]
fn fail_save_before_replace() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
fn fail_save_before_replace() -> Result<(), String> {
    if FAIL_SAVE_BEFORE_REPLACE.swap(false, Ordering::Relaxed) {
        return Err("Injected delete journal save failure before replace".into());
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn fail_next_save_before_replace_for_tests() {
    FAIL_SAVE_BEFORE_REPLACE.store(true, Ordering::Relaxed);
}

fn journal_path(staging_root: &Path) -> PathBuf {
    staging_root.join(DELETE_JOURNAL_FILE)
}

fn unique_staging_relative(staging_root: &Path, journal: &DeleteJournal, relative: &Path) -> PathBuf {
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
        use windows::{
            Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, SetFileAttributesW},
            core::PCWSTR,
        };

        if staging_root
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == super::DELETE_STAGING_DIR)
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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
