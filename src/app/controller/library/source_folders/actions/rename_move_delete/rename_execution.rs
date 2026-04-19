//! Folder rename execution helpers.

use super::*;
use std::fs;

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
            let db = crate::sample_sources::SourceDatabase::open(&source.root)
                .map_err(|err| format!("Database unavailable: {err}"))?;
            let mut batch = db
                .write_batch()
                .map_err(|err| format!("Failed to start database update: {err}"))?;
            let mut entries = Vec::with_capacity(affected.len());
            for entry in &affected {
                let new_relative = new_folder.join(
                    entry.relative_path.strip_prefix(&old_folder).map_err(|_| {
                        format!(
                            "Folder entry missing expected prefix: {}",
                            entry.relative_path.display()
                        )
                    })?,
                );
                batch
                    .remove_file(&entry.relative_path)
                    .map_err(|err| format!("Failed to drop old entry: {err}"))?;
                batch
                    .upsert_file(&new_relative, entry.file_size, entry.modified_ns)
                    .map_err(|err| format!("Failed to register renamed entry: {err}"))?;
                batch
                    .set_tag(&new_relative, entry.tag)
                    .map_err(|err| format!("Failed to copy tag: {err}"))?;
                batch
                    .set_looped(&new_relative, entry.looped)
                    .map_err(|err| format!("Failed to copy loop marker: {err}"))?;
                batch
                    .set_locked(&new_relative, entry.locked)
                    .map_err(|err| format!("Failed to copy keep lock: {err}"))?;
                if let Some(last_played_at) = entry.last_played_at {
                    batch
                        .set_last_played_at(&new_relative, last_played_at)
                        .map_err(|err| format!("Failed to copy playback age: {err}"))?;
                }
                entries.push(WavEntry {
                    relative_path: new_relative,
                    ..entry.clone()
                });
            }
            batch
                .commit()
                .map_err(|err| format!("Failed to save folder rename: {err}"))?;
            Ok(entries)
        });
    FolderRenameResult {
        source_id: source.id,
        old_folder,
        new_folder,
        entries: result.clone().unwrap_or_default(),
        result: result.map(|_| ()),
    }
}
