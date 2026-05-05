//! Folder rename execution helpers.

use super::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Execute the background folder rename job and mirror the rename into the source database.
pub(super) fn run_folder_rename_job(
    source: SampleSource,
    old_folder: PathBuf,
    new_folder: PathBuf,
    affected: Vec<WavEntry>,
    cancel: Arc<AtomicBool>,
) -> FolderRenameResult {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return FolderRenameResult {
            source_id: source.id,
            old_folder,
            new_folder,
            entries: Vec::new(),
            result: Err(String::from("Folder rename cancelled")),
        };
    }
    let absolute_old = source.root.join(&old_folder);
    let absolute_new = source.root.join(&new_folder);
    let result = fs::rename(&absolute_old, &absolute_new)
        .map_err(|err| format!("Failed to rename folder: {err}"))
        .and_then(|_| {
            let result = rewrite_folder_entries(&source.root, &old_folder, &new_folder, &affected);
            match result {
                Ok(entries) => Ok(entries),
                Err(err) => rollback_folder_rename(&absolute_old, &absolute_new, err),
            }
        });
    FolderRenameResult {
        source_id: source.id,
        old_folder,
        new_folder,
        entries: result.clone().unwrap_or_default(),
        result: result.map(|_| ()),
    }
}

fn rewrite_folder_entries(
    source_root: &Path,
    old_folder: &Path,
    new_folder: &Path,
    affected: &[WavEntry],
) -> Result<Vec<WavEntry>, String> {
    let db = crate::sample_sources::SourceDatabase::open(source_root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to start database update: {err}"))?;
    let mut entries = Vec::with_capacity(affected.len());
    for entry in affected {
        entries.push(rewrite_folder_entry(
            &mut batch, old_folder, new_folder, entry,
        )?);
    }
    batch
        .commit()
        .map_err(|err| format!("Failed to save folder rename: {err}"))?;
    Ok(entries)
}

fn rewrite_folder_entry(
    batch: &mut crate::sample_sources::db::SourceWriteBatch<'_>,
    old_folder: &Path,
    new_folder: &Path,
    entry: &WavEntry,
) -> Result<WavEntry, String> {
    let new_relative =
        new_folder.join(entry.relative_path.strip_prefix(old_folder).map_err(|_| {
            format!(
                "Folder entry missing expected prefix: {}",
                entry.relative_path.display()
            )
        })?);
    if let Some(content_hash) = entry.content_hash.as_deref() {
        batch
            .upsert_file_with_hash(
                &new_relative,
                entry.file_size,
                entry.modified_ns,
                content_hash,
            )
            .map_err(|err| format!("Failed to register renamed entry: {err}"))?;
    } else {
        batch
            .upsert_file(&new_relative, entry.file_size, entry.modified_ns)
            .map_err(|err| format!("Failed to register renamed entry: {err}"))?;
    }
    batch
        .set_tag(&new_relative, entry.tag)
        .map_err(|err| format!("Failed to copy tag: {err}"))?;
    batch
        .set_looped(&new_relative, entry.looped)
        .map_err(|err| format!("Failed to copy loop marker: {err}"))?;
    batch
        .set_locked(&new_relative, entry.locked)
        .map_err(|err| format!("Failed to copy keep lock: {err}"))?;
    batch
        .set_missing(&new_relative, entry.missing)
        .map_err(|err| format!("Failed to copy missing marker: {err}"))?;
    batch
        .set_sound_type(&new_relative, entry.sound_type)
        .map_err(|err| format!("Failed to copy sound type: {err}"))?;
    batch
        .set_user_tag(&new_relative, entry.user_tag.as_deref())
        .map_err(|err| format!("Failed to copy custom tag: {err}"))?;
    if let Some(last_played_at) = entry.last_played_at {
        batch
            .set_last_played_at(&new_relative, last_played_at)
            .map_err(|err| format!("Failed to copy playback age: {err}"))?;
    }
    batch
        .copy_tags_between_paths(&entry.relative_path, &new_relative)
        .map_err(|err| format!("Failed to copy normal tags: {err}"))?;
    batch
        .remove_file(&entry.relative_path)
        .map_err(|err| format!("Failed to drop old entry: {err}"))?;
    batch
        .remap_analysis_sample_identity(&entry.relative_path, &new_relative)
        .map_err(|err| format!("Failed to preserve analysis artifacts: {err}"))?;
    Ok(WavEntry {
        relative_path: new_relative,
        ..entry.clone()
    })
}

fn rollback_folder_rename(
    absolute_old: &Path,
    absolute_new: &Path,
    message: String,
) -> Result<Vec<WavEntry>, String> {
    fs::rename(absolute_new, absolute_old)
        .map_err(|err| format!("{message}; rollback failed: {err}"))?;
    Err(message)
}
