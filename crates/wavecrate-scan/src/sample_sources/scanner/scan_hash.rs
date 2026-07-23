use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::sample_sources::db::{PendingRenameEntry, SourceWriteBatch, WavEntry};
use crate::sample_sources::{SourceDatabase, is_supported_audio};

use super::scan::{ChangedSample, RenamedSample, ScanError, ScanStats, UpdatedSample};
use super::scan_fs::{compute_content_hash, ensure_root_dir, read_facts};
use super::scan_writer::{ScanWritePhase, ScanWriter, UncoordinatedScanWriter};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HashBackfill {
    relative_path: PathBuf,
    file_size: u64,
    modified_ns: i64,
    content_hash: String,
    file_identity: Option<String>,
}

const META_CONTENT_AUDIT_CURSOR: &str = "source_content_audit_cursor_v1";

fn cancel_requested(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed))
}

pub(super) fn verify_content_batch(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    max_hashes: usize,
    audit_completed_at: Option<i64>,
) -> Result<ScanStats, ScanError> {
    let manifest_before = super::manifest::capture_manifest(db)?;
    let root = ensure_root_dir(db)?;
    let entries = db.list_manifest_entries()?;
    let cursor = db
        .get_metadata(META_CONTENT_AUDIT_CURSOR)?
        .unwrap_or_default();
    let start = entries
        .iter()
        .position(|entry| entry.relative_path.to_string_lossy().as_ref() > cursor.as_str())
        .unwrap_or(0);
    let selected = entries
        .iter()
        .cycle()
        .skip(start)
        .take(max_hashes.min(entries.len()))
        .cloned()
        .collect::<Vec<_>>();
    let mut stats = ScanStats::default();
    let mut verified = Vec::new();
    for entry in &selected {
        if cancel_requested(cancel) {
            return Err(ScanError::Canceled);
        }
        let absolute = root.join(&entry.relative_path);
        if !is_supported_regular_audio_file(&absolute) {
            continue;
        }
        let before_hash = read_facts(&root, &absolute)?;
        let content_hash = compute_content_hash(&absolute, cancel)?;
        let after_hash = read_facts(&root, &absolute)?;
        if !before_hash.same_content_snapshot(&after_hash) {
            continue;
        }
        verified.push((entry.clone(), after_hash, content_hash));
        stats.hashes_computed += 1;
    }
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let committed_snapshot = if !selected.is_empty() || audit_completed_at.is_some() {
        let mut batch = db.write_batch()?;
        for (previous, facts, content_hash) in &verified {
            if previous.content_hash.as_deref() == Some(content_hash.as_str())
                && previous.file_size == facts.size
                && previous.modified_ns == facts.modified_ns
                && previous.file_identity == facts.file_identity
            {
                continue;
            }
            if previous.file_identity != facts.file_identity {
                tracing::debug!(
                    path = %previous.relative_path.display(),
                    previous_identity = ?previous.file_identity,
                    current_identity = ?facts.file_identity,
                    "Source content audit refreshed filesystem identity"
                );
            }
            batch.upsert_file_with_hash(
                &previous.relative_path,
                facts.size,
                facts.modified_ns,
                content_hash,
            )?;
            batch.set_file_identity(&previous.relative_path, facts.file_identity.as_deref())?;
            stats.updated += 1;
            stats.updated_samples.push(UpdatedSample {
                relative_path: previous.relative_path.clone(),
                file_size: facts.size,
                modified_ns: facts.modified_ns,
                content_hash: Some(content_hash.clone()),
            });
            if previous.content_hash.as_deref() != Some(content_hash.as_str()) {
                stats.content_changed += 1;
                stats.changed_samples.push(ChangedSample {
                    relative_path: previous.relative_path.clone(),
                    file_size: facts.size,
                    modified_ns: facts.modified_ns,
                    content_hash: content_hash.clone(),
                });
            }
        }
        if let Some(last) = selected.last() {
            batch.set_metadata(
                META_CONTENT_AUDIT_CURSOR,
                last.relative_path.to_string_lossy().as_ref(),
            )?;
        }
        if let Some(completed_at) = audit_completed_at {
            batch.complete_manifest_audit(completed_at)?;
        }
        batch.commit_with_manifest_snapshot()?
    } else {
        db.manifest_snapshot_with_revision()?
    };
    super::manifest::publish_committed_delta(&mut stats, manifest_before, committed_snapshot);
    Ok(stats)
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
    exact_path: Option<&std::path::Path>,
) -> Result<ScanStats, ScanError> {
    deep_hash_scan_with_writer(
        db,
        cancel,
        rename_candidates,
        scope,
        max_hashes,
        exact_path,
        &UncoordinatedScanWriter,
    )
}

pub(super) fn deep_hash_scan_with_writer(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    rename_candidates: &HashSet<PathBuf>,
    scope: DeferredHashScope,
    max_hashes: Option<usize>,
    exact_path: Option<&std::path::Path>,
    writer: &impl ScanWriter,
) -> Result<ScanStats, ScanError> {
    deep_hash_scan_with_post_hash_hook(
        db,
        cancel,
        rename_candidates,
        scope,
        max_hashes,
        exact_path,
        writer,
        |_| {},
    )
}

pub(super) fn reconcile_hashed_rename_candidates_with_writer(
    db: &SourceDatabase,
    rename_candidates: &HashSet<PathBuf>,
    cancel: Option<&AtomicBool>,
    writer: &impl ScanWriter,
) -> Result<Vec<RenamedSample>, ScanError> {
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let root = ensure_root_dir(db)?;
    let rename_candidates = rename_candidates.clone();
    if rename_candidates.is_empty() {
        return Ok(Vec::new());
    }

    let entries_by_path = db
        .list_files()?
        .into_iter()
        .filter(|entry| {
            !entry.missing && is_supported_regular_audio_file(&root.join(&entry.relative_path))
        })
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect::<HashMap<_, _>>();
    let manifest_entries = db.list_manifest_entries()?;
    let pending_entries = db
        .list_pending_renames()?
        .into_iter()
        .filter(|entry| !root.join(&entry.relative_path).exists())
        .collect::<Vec<_>>();
    if pending_entries.is_empty() {
        return Ok(Vec::new());
    }

    let mut present_by_hash = HashMap::new();
    let mut pending_by_hash = HashMap::new();
    let mut present_by_file_identity = HashMap::new();
    let mut pending_by_file_identity = HashMap::new();
    for entry in manifest_entries {
        if !entries_by_path.contains_key(&entry.relative_path) {
            continue;
        }
        if let Some(hash) = entry.content_hash.as_deref() {
            present_by_hash
                .entry(hash.to_string())
                .or_insert_with(Vec::new)
                .push(entry.relative_path.clone());
        }
        if rename_candidates.contains(&entry.relative_path)
            && let Some(file_identity) = entry.file_identity.as_deref()
        {
            present_by_file_identity
                .entry(file_identity.to_string())
                .or_insert_with(Vec::new)
                .push(entry.relative_path.clone());
        }
    }
    for entry in pending_entries {
        if let Some(hash) = entry.content_hash.as_deref() {
            pending_by_hash
                .entry(hash.to_string())
                .or_insert_with(Vec::new)
                .push(entry.clone());
        }
        if let Some(file_identity) = entry.file_identity.as_deref() {
            pending_by_file_identity
                .entry(file_identity.to_string())
                .or_insert_with(Vec::new)
                .push(entry);
        }
    }

    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let _writer = writer.lock(ScanWritePhase::DeferredHash);
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let mut batch = db.write_batch()?;
    let retained_candidates = retain_matching_rename_candidates(
        &mut batch,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
    )?;
    let mut renamed_samples = reconcile_same_file_renames(
        &mut batch,
        &entries_by_path,
        &present_by_file_identity,
        &pending_by_file_identity,
        &HashSet::new(),
    )?;
    let already_reconciled = reconciled_paths(&renamed_samples);
    renamed_samples.extend(reconcile_missing_renames(
        &mut batch,
        &entries_by_path,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
        &already_reconciled,
    )?);
    if renamed_samples.is_empty() && retained_candidates == 0 {
        return Ok(renamed_samples);
    }
    batch.commit()?;
    Ok(renamed_samples)
}

fn deep_hash_scan_with_post_hash_hook(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    rename_candidates: &HashSet<PathBuf>,
    scope: DeferredHashScope,
    max_hashes: Option<usize>,
    exact_path: Option<&std::path::Path>,
    writer: &impl ScanWriter,
    mut post_hash: impl FnMut(&std::path::Path),
) -> Result<ScanStats, ScanError> {
    let manifest_before = super::manifest::capture_manifest(db)?;
    let root = ensure_root_dir(db)?;
    let mut rename_candidates = rename_candidates.clone();
    rename_candidates.extend(db.list_pending_rename_destinations()?);
    let entries = if let Some(exact_path) = exact_path {
        db.entry_for_path(exact_path)?.into_iter().collect()
    } else if scope == DeferredHashScope::AllUnhashed && rename_candidates.is_empty() {
        db.list_pending_hash_files(max_hashes.unwrap_or(usize::MAX))?
    } else {
        db.list_files()?
    };
    let mut entries_by_path: HashMap<PathBuf, WavEntry> = entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect();
    let has_unhashed_files = scope == DeferredHashScope::AllUnhashed
        && entries_by_path.values().any(|entry| {
            !entry.missing
                && entry.content_hash.is_none()
                && root.join(&entry.relative_path).is_file()
        });
    if !has_unhashed_files && rename_candidates.is_empty() {
        let mut stats = ScanStats::default();
        let committed_snapshot = db.manifest_snapshot_with_revision()?;
        super::manifest::publish_committed_delta(&mut stats, manifest_before, committed_snapshot);
        return Ok(stats);
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
        post_hash(&absolute);
        let after_hash = read_facts(&root, &absolute)?;
        if !facts.same_content_snapshot(&after_hash) {
            continue;
        }
        hash_backfills.push(HashBackfill {
            relative_path: entry.relative_path.clone(),
            file_size: after_hash.size,
            modified_ns: after_hash.modified_ns,
            content_hash: hash.clone(),
            file_identity: after_hash.file_identity,
        });
        entry.file_size = after_hash.size;
        entry.modified_ns = after_hash.modified_ns;
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

    let _writer = writer.lock(ScanWritePhase::DeferredHash);
    if cancel_requested(cancel) {
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
    retain_matching_rename_candidates(
        &mut batch,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
    )?;

    let mut renamed_samples = reconcile_same_file_renames(
        &mut batch,
        &entries_by_path,
        &present_by_file_identity,
        &pending_by_file_identity,
        &HashSet::new(),
    )?;
    let already_reconciled = reconciled_paths(&renamed_samples);
    renamed_samples.extend(reconcile_missing_renames(
        &mut batch,
        &entries_by_path,
        &present_by_hash,
        &pending_by_hash,
        &rename_candidates,
        &already_reconciled,
    )?);
    stats.renames_reconciled = renamed_samples.len();
    stats.renamed_samples = renamed_samples;

    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let committed_snapshot = batch.commit_with_manifest_snapshot()?;
    super::manifest::publish_committed_delta(&mut stats, manifest_before, committed_snapshot);
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
    already_reconciled: &HashSet<PathBuf>,
) -> Result<Vec<RenamedSample>, ScanError> {
    let mut reconciled = Vec::new();
    for (hash, pending_entries) in pending_by_hash {
        let [pending_entry] = pending_entries.as_slice() else {
            continue;
        };
        if already_reconciled.contains(&pending_entry.relative_path) {
            continue;
        }
        let Some(present_paths) = present_by_hash.get(hash) else {
            continue;
        };
        let candidates = present_paths
            .iter()
            .filter(|path| rename_candidates.contains(*path) && !already_reconciled.contains(*path))
            .collect::<Vec<_>>();
        let [present_path] = candidates.as_slice() else {
            continue;
        };
        let present_path = *present_path;
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

fn retain_matching_rename_candidates(
    batch: &mut SourceWriteBatch<'_>,
    present_by_hash: &HashMap<String, Vec<PathBuf>>,
    pending_by_hash: &HashMap<String, Vec<PendingRenameEntry>>,
    rename_candidates: &HashSet<PathBuf>,
) -> Result<usize, ScanError> {
    let mut retained = 0;
    for hash in pending_by_hash.keys() {
        let Some(present_paths) = present_by_hash.get(hash) else {
            continue;
        };
        for path in present_paths
            .iter()
            .filter(|path| rename_candidates.contains(*path))
        {
            batch.retain_pending_rename_destination(path, hash)?;
            retained += 1;
        }
    }
    Ok(retained)
}

fn reconciled_paths(renamed_samples: &[RenamedSample]) -> HashSet<PathBuf> {
    renamed_samples
        .iter()
        .flat_map(|renamed| {
            [
                renamed.old_relative_path.clone(),
                renamed.new_relative_path.clone(),
            ]
        })
        .collect()
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
            None,
        );

        assert!(matches!(result, Err(ScanError::Canceled)));
    }

    #[test]
    fn deep_hash_scan_bounds_a_large_library_batch() {
        let dir = tempfile::tempdir().expect("temp source");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        for index in 0..512 {
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
            Some(8),
            None,
        )
        .expect("bounded hash pass");

        assert_eq!(stats.hashes_computed, 8);
        assert_eq!(
            db.list_files()
                .expect("list files")
                .iter()
                .filter(|entry| entry.content_hash.is_some())
                .count(),
            8
        );
    }

    #[test]
    fn deep_hash_scan_exact_path_does_not_process_earlier_pending_rows() {
        let dir = tempfile::tempdir().expect("temp source");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        for relative in [Path::new("a-first.wav"), Path::new("z-target.wav")] {
            std::fs::write(dir.path().join(relative), [9_u8; 32]).expect("write wav");
            db.upsert_file(relative, 32, 1).expect("insert pending row");
        }

        let stats = deep_hash_scan(
            &db,
            None,
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            Some(1),
            Some(Path::new("z-target.wav")),
        )
        .expect("targeted hash pass");

        assert_eq!(stats.hashes_computed, 1);
        assert!(
            db.entry_for_path(Path::new("a-first.wav"))
                .unwrap()
                .unwrap()
                .content_hash
                .is_none()
        );
        assert!(
            db.entry_for_path(Path::new("z-target.wav"))
                .unwrap()
                .unwrap()
                .content_hash
                .is_some()
        );
    }

    #[test]
    fn deep_hash_scan_does_not_commit_a_file_mutated_during_hashing() {
        let dir = tempfile::tempdir().expect("temp source");
        let relative = Path::new("changing.wav");
        let absolute = dir.path().join(relative);
        std::fs::write(&absolute, [1_u8; 32]).expect("write initial wav");
        let original_modified = std::fs::metadata(&absolute)
            .expect("read initial metadata")
            .modified()
            .expect("read initial modified time");
        let db = SourceDatabase::open_for_source_write(dir.path()).expect("source db");
        db.upsert_file(relative, 32, 1).expect("insert pending row");

        let stats = deep_hash_scan_with_post_hash_hook(
            &db,
            None,
            &HashSet::new(),
            DeferredHashScope::AllUnhashed,
            Some(1),
            Some(relative),
            &UncoordinatedScanWriter,
            |path| {
                std::fs::write(path, [2_u8; 32]).expect("mutate during hashing");
                let file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(path)
                    .expect("reopen mutated wav");
                file.set_times(std::fs::FileTimes::new().set_modified(original_modified))
                    .expect("restore modified time");
            },
        )
        .expect("defer unstable hash");

        assert_eq!(stats.hashes_computed, 0);
        assert!(
            db.entry_for_path(relative)
                .expect("read pending row")
                .expect("pending row")
                .content_hash
                .is_none(),
            "an unstable read must remain pending for a later hash pass"
        );
    }
}
