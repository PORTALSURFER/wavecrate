use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::sample_sources::db::{PendingRenameEntry, SourceWriteBatch, WavEntry};
use crate::sample_sources::{SourceDatabase, is_supported_audio};

use super::scan::{RenamedSample, ScanError, ScanStats};
use super::scan_fs::{compute_content_hash, ensure_root_dir, read_facts};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HashBackfill {
    relative_path: PathBuf,
    file_size: u64,
    modified_ns: i64,
    content_hash: String,
    file_identity: Option<String>,
}

fn is_supported_regular_audio_file(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
        && is_supported_audio(path)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeferredHashScope {
    AllUnhashed,
    RenameCandidates,
}

pub(super) fn deep_hash_scan(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    rename_candidates: &HashSet<PathBuf>,
    scope: DeferredHashScope,
    max_hashes: Option<usize>,
) -> Result<ScanStats, ScanError> {
    let root = ensure_root_dir(db)?;
    let entries = db.list_files()?;
    let mut entries_by_path: HashMap<PathBuf, WavEntry> = entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect();
    let mut rename_candidates = rename_candidates.clone();
    rename_candidates.extend(db.list_pending_rename_destinations()?);
    let has_unhashed_files = scope == DeferredHashScope::AllUnhashed
        && entries_by_path.values().any(|entry| {
            !entry.missing
                && entry.content_hash.is_none()
                && root.join(&entry.relative_path).is_file()
        });
    if !has_unhashed_files && rename_candidates.is_empty() {
        return Ok(ScanStats::default());
    }
    let pending_entries = db.list_pending_renames()?;
    let mut stats = ScanStats::default();
    let mut present_by_hash = HashMap::new();
    let mut pending_by_hash = HashMap::new();
    let mut present_by_file_identity = HashMap::new();
    let mut pending_by_file_identity = HashMap::new();
    let mut hash_backfills = Vec::new();

    for entry in entries_by_path.values() {
        if entry.missing
            || rename_candidates.contains(&entry.relative_path)
            || !is_supported_regular_audio_file(&root.join(&entry.relative_path))
        {
            continue;
        }
        let Some(hash) = entry.content_hash.as_deref() else {
            continue;
        };
        present_by_hash
            .entry(hash.to_string())
            .or_insert_with(Vec::new)
            .push(entry.relative_path.clone());
    }
    for entry in pending_entries {
        if let Some(file_identity) = entry.file_identity.as_deref() {
            pending_by_file_identity
                .entry(file_identity.to_string())
                .or_insert_with(Vec::new)
                .push(entry.clone());
        }
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
        if entry.missing {
            continue;
        }
        let is_rename_candidate = rename_candidates.contains(&entry.relative_path);
        if entry.content_hash.is_some() && !is_rename_candidate {
            continue;
        }
        if scope == DeferredHashScope::RenameCandidates && !is_rename_candidate {
            continue;
        }
        let was_unhashed = entry.content_hash.is_none();
        if was_unhashed
            && !is_rename_candidate
            && max_hashes.is_some_and(|limit| stats.hashes_computed >= limit)
        {
            continue;
        }
        let absolute = root.join(&entry.relative_path);
        if !is_supported_regular_audio_file(&absolute) {
            continue;
        }
        let facts = read_facts(&root, &absolute)?;
        let hash = compute_content_hash(&absolute, cancel)?;
        hash_backfills.push(HashBackfill {
            relative_path: entry.relative_path.clone(),
            file_size: facts.size,
            modified_ns: facts.modified_ns,
            content_hash: hash.clone(),
            file_identity: facts.file_identity,
        });
        entry.file_size = facts.size;
        entry.modified_ns = facts.modified_ns;
        entry.content_hash = Some(hash.clone());
        present_by_hash
            .entry(hash)
            .or_insert_with(Vec::new)
            .push(entry.relative_path.clone());
        if was_unhashed {
            stats.hashes_computed += 1;
        }
    }

    for backfill in &hash_backfills {
        if !rename_candidates.contains(&backfill.relative_path) {
            continue;
        }
        let Some(file_identity) = backfill.file_identity.as_ref() else {
            continue;
        };
        present_by_file_identity
            .entry(file_identity.clone())
            .or_insert_with(Vec::new)
            .push(backfill.relative_path.clone());
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
        batch.set_file_identity(&backfill.relative_path, backfill.file_identity.as_deref())?;
    }

    let mut renamed_samples = reconcile_missing_renames(
        &mut batch,
        &entries_by_path,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
    )?;
    let already_reconciled = renamed_samples
        .iter()
        .flat_map(|renamed| {
            [
                renamed.old_relative_path.clone(),
                renamed.new_relative_path.clone(),
            ]
        })
        .collect::<HashSet<_>>();
    renamed_samples.extend(reconcile_same_file_renames(
        &mut batch,
        &entries_by_path,
        &present_by_file_identity,
        &pending_by_file_identity,
        &already_reconciled,
    )?);
    stats.renames_reconciled = renamed_samples.len();
    stats.renamed_samples = renamed_samples;

    batch.commit()?;
    Ok(stats)
}

fn reconcile_same_file_renames(
    batch: &mut SourceWriteBatch<'_>,
    entries_by_path: &HashMap<PathBuf, WavEntry>,
    present_by_file_identity: &HashMap<String, Vec<PathBuf>>,
    pending_by_file_identity: &HashMap<String, Vec<PendingRenameEntry>>,
    already_reconciled: &HashSet<PathBuf>,
) -> Result<Vec<RenamedSample>, ScanError> {
    let mut reconciled = Vec::new();
    for (file_identity, pending_entries) in pending_by_file_identity {
        let [pending_entry] = pending_entries.as_slice() else {
            continue;
        };
        let Some(present_paths) = present_by_file_identity.get(file_identity) else {
            continue;
        };
        let [present_path] = present_paths.as_slice() else {
            continue;
        };
        if pending_entry.relative_path == *present_path
            || already_reconciled.contains(&pending_entry.relative_path)
            || already_reconciled.contains(present_path)
        {
            continue;
        }
        let Some(present_entry) = entries_by_path.get(present_path) else {
            continue;
        };
        if present_entry.file_size != pending_entry.file_size
            || present_entry.modified_ns != pending_entry.modified_ns
        {
            continue;
        }
        let Some(hash) = present_entry.content_hash.as_deref() else {
            continue;
        };
        apply_deep_rename(batch, present_entry, pending_entry, hash)?;
        reconciled.push(RenamedSample {
            old_relative_path: pending_entry.relative_path.clone(),
            new_relative_path: present_entry.relative_path.clone(),
            file_size: present_entry.file_size,
            modified_ns: present_entry.modified_ns,
            content_hash: Some(hash.to_string()),
        });
    }
    Ok(reconciled)
}

fn reconcile_missing_renames(
    batch: &mut SourceWriteBatch<'_>,
    entries_by_path: &HashMap<PathBuf, WavEntry>,
    present_by_hash: &HashMap<String, Vec<PathBuf>>,
    pending_by_hash: &HashMap<String, Vec<PendingRenameEntry>>,
    rename_candidates: &HashSet<PathBuf>,
) -> Result<Vec<RenamedSample>, ScanError> {
    let mut reconciled = Vec::new();
    for (hash, pending_entries) in pending_by_hash {
        if pending_entries.len() != 1 {
            continue;
        }
        let Some(present_paths) = present_by_hash.get(hash) else {
            continue;
        };
        let candidates = present_paths
            .iter()
            .filter(|path| rename_candidates.contains(*path))
            .collect::<Vec<_>>();
        let present_path = if present_paths.len() == 1 && candidates.len() == 1 {
            candidates[0]
        } else {
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
            if !rename_candidates.contains(*present_path) {
                continue;
            }
            *present_path
        };
        if pending_entries[0].relative_path == *present_path {
            continue;
        }
        let pending_entry = &pending_entries[0];
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
    batch.clear_pending_rename_destination(&present_entry.relative_path)?;
    batch.upsert_file_with_hash_and_tag(
        &present_entry.relative_path,
        present_entry.file_size,
        present_entry.modified_ns,
        hash,
        pending_entry.metadata.tag,
        false,
    )?;
    batch.restore_rename_metadata(&present_entry.relative_path, &pending_entry.metadata)?;
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

        let result = deep_hash_scan(
            &db,
            Some(&cancel),
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            None,
        );

        assert!(matches!(result, Err(ScanError::Canceled)));
    }

    #[test]
    fn deep_hash_scan_respects_non_rename_batch_limit() {
        let dir = tempfile::tempdir().expect("temp source");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        for index in 0..5 {
            let relative = PathBuf::from(format!("pending-{index}.wav"));
            std::fs::write(dir.path().join(&relative), [index as u8; 32]).expect("write wav");
            db.upsert_file(&relative, 32, index)
                .expect("insert pending row");
        }

        let stats = deep_hash_scan(
            &db,
            None,
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            Some(2),
        )
        .expect("bounded hash pass");

        assert_eq!(stats.hashes_computed, 2);
        assert_eq!(
            db.list_files()
                .expect("list files")
                .iter()
                .filter(|entry| entry.content_hash.is_some())
                .count(),
            2
        );
    }
}
