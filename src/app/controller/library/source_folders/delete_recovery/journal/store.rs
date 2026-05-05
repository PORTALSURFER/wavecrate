use super::DeleteJournal;
use super::DeleteJournalEntry;
use super::DeleteJournalStage;
use super::atomic_save::{fail_save_before_replace, replace_journal_file};
use super::cleanup_staging_root;
use std::fs;
use std::path::{Path, PathBuf};

const DELETE_JOURNAL_FILE: &str = "delete_journal.json";

pub(crate) fn load_journal(staging_root: &Path) -> Result<DeleteJournal, String> {
    let path = journal_path(staging_root);
    if !path.exists() {
        return Ok(DeleteJournal::default());
    }
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read delete journal: {err}"))?;
    serde_json::from_slice(&bytes).map_err(|err| format!("Failed to parse delete journal: {err}"))
}

pub(crate) fn remove_entry(staging_root: &Path, id: &str) -> Result<(), String> {
    let mut journal = load_journal(staging_root)?;
    let before = journal.entries.len();
    journal.entries.retain(|entry| entry.id != id);
    if journal.entries.len() == before {
        return Err("Delete journal entry missing".into());
    }
    save_journal(staging_root, &journal)
}

#[cfg(test)]
pub(crate) fn update_entry_stage(
    staging_root: &Path,
    id: &str,
    stage: DeleteJournalStage,
) -> Result<(), String> {
    update_journal_stage(staging_root, id, stage)
}

pub(super) fn update_journal_stage(
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

pub(super) fn insert_entry(staging_root: &Path, entry: DeleteJournalEntry) -> Result<(), String> {
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

pub(super) fn update_entry(
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
    fail_save_before_replace(&path)?;
    replace_journal_file(&tmp_path, &path)?;
    Ok(())
}

pub(super) fn journal_path(staging_root: &Path) -> PathBuf {
    staging_root.join(DELETE_JOURNAL_FILE)
}
