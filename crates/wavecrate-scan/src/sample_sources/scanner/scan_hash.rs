use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::{PendingRenameEntry, SourceWriteBatch, WavEntry};

use super::scan::{RenamedSample, ScanError, ScanStats};
use super::scan_fs::{compute_content_hash, ensure_root_dir, read_facts};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HashBackfill {
    relative_path: PathBuf,
    file_size: u64,
    modified_ns: i64,
    content_hash: String,
}

pub(super) fn deep_hash_scan(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
) -> Result<ScanStats, ScanError> {
    let root = ensure_root_dir(db)?;
    let entries = db.list_files()?;
    let mut entries_by_path: HashMap<PathBuf, WavEntry> = entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect();
    let pending_entries = db.list_pending_renames()?;
    let mut stats = ScanStats::default();
    let mut present_by_hash = HashMap::new();
    let mut pending_by_hash = HashMap::new();
    let mut hash_backfills = Vec::new();

    for entry in entries_by_path.values() {
        let Some(hash) = entry.content_hash.as_deref() else {
            continue;
        };
        present_by_hash
            .entry(hash.to_string())
            .or_insert_with(Vec::new)
            .push(entry.relative_path.clone());
    }
    for entry in pending_entries {
        let Some(hash) = entry.content_hash.as_deref() else {
            continue;
        };
        pending_by_hash
            .entry(hash.to_string())
            .or_insert_with(Vec::new)
            .push(entry);
    }

    for entry in entries_by_path.values_mut() {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        if entry.missing || entry.content_hash.is_some() {
            continue;
        }
        let absolute = root.join(&entry.relative_path);
        if !absolute.exists() {
            continue;
        }
        let facts = read_facts(&root, &absolute)?;
        let hash = compute_content_hash(&absolute, cancel)?;
        hash_backfills.push(HashBackfill {
            relative_path: entry.relative_path.clone(),
            file_size: facts.size,
            modified_ns: facts.modified_ns,
            content_hash: hash.clone(),
        });
        entry.file_size = facts.size;
        entry.modified_ns = facts.modified_ns;
        entry.content_hash = Some(hash.clone());
        present_by_hash
            .entry(hash)
            .or_insert_with(Vec::new)
            .push(entry.relative_path.clone());
        stats.hashes_computed += 1;
    }

    if let Some(cancel) = cancel
        && cancel.load(Ordering::Relaxed)
    {
        return Err(ScanError::Canceled);
    }

    let mut batch = db.write_batch()?;
    for backfill in &hash_backfills {
        batch.upsert_file_with_hash(
            &backfill.relative_path,
            backfill.file_size,
            backfill.modified_ns,
            &backfill.content_hash,
        )?;
    }

    let renamed_samples = reconcile_missing_renames(
        &mut batch,
        &entries_by_path,
        &present_by_hash,
        &pending_by_hash,
    )?;
    stats.renames_reconciled = renamed_samples.len();
    stats.renamed_samples = renamed_samples;

    batch.commit()?;
    Ok(stats)
}

fn reconcile_missing_renames(
    batch: &mut SourceWriteBatch<'_>,
    entries_by_path: &HashMap<PathBuf, WavEntry>,
    present_by_hash: &HashMap<String, Vec<PathBuf>>,
    pending_by_hash: &HashMap<String, Vec<PendingRenameEntry>>,
) -> Result<Vec<RenamedSample>, ScanError> {
    let mut reconciled = Vec::new();
    for (hash, pending_entries) in pending_by_hash {
        if pending_entries.len() != 1 {
            continue;
        }
        let Some(present_paths) = present_by_hash.get(hash) else {
            continue;
        };
        let matching_facts = present_paths
            .iter()
            .filter(|path| {
                entries_by_path.get(*path).is_some_and(|entry| {
                    entry.file_size == pending_entries[0].file_size
                        && entry.modified_ns == pending_entries[0].modified_ns
                })
            })
            .collect::<Vec<_>>();
        let [present_path] = matching_facts.as_slice() else {
            continue;
        };
        let present_path = *present_path;
        let pending_entry = &pending_entries[0];
        if pending_entry.relative_path == *present_path {
            continue;
        }
        let Some(present_entry) = entries_by_path.get(present_path) else {
            continue;
        };
        apply_deep_rename(batch, present_entry, pending_entry, hash)?;
        reconciled.push(RenamedSample {
            old_relative_path: pending_entry.relative_path.clone(),
            new_relative_path: present_entry.relative_path.clone(),
            file_size: present_entry.file_size,
            modified_ns: present_entry.modified_ns,
            content_hash: Some(hash.clone()),
        });
    }
    Ok(reconciled)
}

fn apply_deep_rename(
    batch: &mut SourceWriteBatch<'_>,
    present_entry: &WavEntry,
    pending_entry: &PendingRenameEntry,
    hash: &str,
) -> Result<(), ScanError> {
    batch.clear_pending_rename(&pending_entry.relative_path)?;
    batch.upsert_file_with_hash_and_tag(
        &present_entry.relative_path,
        present_entry.file_size,
        present_entry.modified_ns,
        hash,
        pending_entry.tag,
        false,
    )?;
    if pending_entry.looped {
        batch.set_looped(&present_entry.relative_path, pending_entry.looped)?;
    }
    if pending_entry.sound_type.is_some() {
        batch.set_sound_type(&present_entry.relative_path, pending_entry.sound_type)?;
    }
    if pending_entry.locked {
        batch.set_locked(&present_entry.relative_path, pending_entry.locked)?;
    }
    if let Some(last_played_at) = pending_entry.last_played_at {
        batch.set_last_played_at(&present_entry.relative_path, last_played_at)?;
    }
    if pending_entry.user_tag.is_some() {
        batch.set_user_tag(
            &present_entry.relative_path,
            pending_entry.user_tag.as_deref(),
        )?;
    }
    batch.set_tag_named(&present_entry.relative_path, pending_entry.tag_named)?;
    batch.replace_tags_for_path(&present_entry.relative_path, &pending_entry.normal_tags)?;
    batch.remap_analysis_sample_identity(
        &pending_entry.relative_path,
        &present_entry.relative_path,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::atomic::AtomicBool};

    use crate::sample_sources::SourceDatabase;

    use super::*;

    #[test]
    fn deep_hash_scan_checks_cancel_before_writer_lock() {
        let dir = tempfile::tempdir().expect("temp source");
        std::fs::write(dir.path().join("pending.wav"), b"pending").expect("write wav");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        db.upsert_file(Path::new("pending.wav"), 7, 10)
            .expect("file row");
        let lock_db = SourceDatabase::open_for_source_write(dir.path()).expect("lock db");
        let _writer = lock_db.write_batch().expect("writer lock");
        let cancel = AtomicBool::new(true);

        let result = deep_hash_scan(&db, Some(&cancel));

        assert!(matches!(result, Err(ScanError::Canceled)));
    }
}
