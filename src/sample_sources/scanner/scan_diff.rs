use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::{SourceWriteBatch, WavEntry};

use super::scan::{ChangedSample, ScanContext, ScanError, ScanMode, ScanStats};
use super::scan_fs::{FileFacts, compute_content_hash};

const QUICK_HASH_MAX_BYTES: u64 = 8 * 1024 * 1024;

pub(super) fn apply_diff(
    db: &SourceDatabase,
    batch: &mut SourceWriteBatch<'_>,
    facts: FileFacts,
    context: &mut ScanContext,
    root: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<(), ScanError> {
    let path = facts.relative.clone();
    let should_hash = should_compute_full_hash(context.mode, facts.size);
    match context.existing.remove(&path) {
        Some(entry) if entry.file_size == facts.size && entry.modified_ns == facts.modified_ns => {
            if entry.missing {
                batch.set_missing(&path, false)?;
            }
            if entry.content_hash.is_none() {
                if should_hash {
                    let absolute = root.join(&path);
                    let hash = compute_content_hash(&absolute, cancel)?;
                    batch.upsert_file_with_hash(&path, facts.size, facts.modified_ns, &hash)?;
                    context.stats.hashes_computed += 1;
                } else {
                    context.stats.hashes_pending += 1;
                }
            }
        }
        Some(entry) => {
            let absolute = root.join(&path);
            let previous_hash = entry.content_hash.as_deref();
            if should_hash {
                let hash = compute_content_hash(&absolute, cancel)?;
                batch.upsert_file_with_hash(&path, facts.size, facts.modified_ns, &hash)?;
                context.stats.hashes_computed += 1;
                if previous_hash != Some(hash.as_str()) {
                    context.stats.content_changed += 1;
                    context.stats.changed_samples.push(ChangedSample {
                        relative_path: path.clone(),
                        file_size: facts.size,
                        modified_ns: facts.modified_ns,
                        content_hash: hash,
                    });
                }
            } else {
                batch.upsert_file_without_hash(&path, facts.size, facts.modified_ns)?;
                context.stats.hashes_pending += 1;
            }
            context.stats.updated += 1;
        }
        None => {
            let absolute = root.join(&path);
            if should_hash {
                let hash = compute_content_hash(&absolute, cancel)?;
                if let Some(entry) = take_rename_candidate_by_hash(db, context, &hash)? {
                    apply_rename(batch, &path, &facts, &hash, entry)?;
                    context.stats.updated += 1;
                    context.stats.renames_reconciled += 1;
                    return Ok(());
                }
                batch.upsert_file_with_hash(&path, facts.size, facts.modified_ns, &hash)?;
                context.stats.added += 1;
                context.stats.content_changed += 1;
                context.stats.hashes_computed += 1;
                context.stats.changed_samples.push(ChangedSample {
                    relative_path: path.clone(),
                    file_size: facts.size,
                    modified_ns: facts.modified_ns,
                    content_hash: hash,
                });
            } else {
                if let Some(entry) =
                    take_rename_candidate_by_facts(db, context, facts.size, facts.modified_ns)?
                {
                    apply_rename_without_hash(batch, &path, &facts, entry)?;
                    context.stats.updated += 1;
                    context.stats.renames_reconciled += 1;
                    context.stats.hashes_pending += 1;
                    return Ok(());
                }
                batch.upsert_file_without_hash(&path, facts.size, facts.modified_ns)?;
                context.stats.added += 1;
                context.stats.hashes_pending += 1;
            }
        }
    }
    Ok(())
}

pub(super) fn mark_missing(
    batch: &mut SourceWriteBatch<'_>,
    existing: HashMap<PathBuf, WavEntry>,
    stats: &mut ScanStats,
    mode: ScanMode,
) -> Result<(), ScanError> {
    for leftover in existing.values() {
        match mode {
            ScanMode::Quick => {
                if leftover.missing {
                    continue;
                }
                batch.set_missing(&leftover.relative_path, true)?;
                stats.missing += 1;
            }
            ScanMode::Hard => {
                batch.remove_file(&leftover.relative_path)?;
                stats.missing += 1;
            }
        }
    }
    Ok(())
}

fn apply_rename(
    batch: &mut SourceWriteBatch<'_>,
    new_path: &Path,
    facts: &FileFacts,
    hash: &str,
    entry: WavEntry,
) -> Result<(), ScanError> {
    batch.remove_file(&entry.relative_path)?;
    batch.upsert_file_with_hash_and_tag(
        new_path,
        facts.size,
        facts.modified_ns,
        hash,
        entry.tag,
        false,
    )?;
    if entry.looped {
        batch.set_looped(new_path, entry.looped)?;
    }
    if entry.locked {
        batch.set_locked(new_path, entry.locked)?;
    }
    if let Some(last_played_at) = entry.last_played_at {
        batch.set_last_played_at(new_path, last_played_at)?;
    }
    Ok(())
}

fn apply_rename_without_hash(
    batch: &mut SourceWriteBatch<'_>,
    new_path: &Path,
    facts: &FileFacts,
    entry: WavEntry,
) -> Result<(), ScanError> {
    batch.remove_file(&entry.relative_path)?;
    batch.upsert_file_without_hash(new_path, facts.size, facts.modified_ns)?;
    batch.set_tag(new_path, entry.tag)?;
    if entry.looped {
        batch.set_looped(new_path, entry.looped)?;
    }
    if entry.locked {
        batch.set_locked(new_path, entry.locked)?;
    }
    if let Some(last_played_at) = entry.last_played_at {
        batch.set_last_played_at(new_path, last_played_at)?;
    }
    Ok(())
}

fn take_rename_candidate_by_hash(
    db: &SourceDatabase,
    context: &mut ScanContext,
    hash: &str,
) -> Result<Option<WavEntry>, ScanError> {
    if let std::collections::hash_map::Entry::Vacant(entry) =
        context.rename_candidates_by_hash.entry(hash.to_string())
    {
        let paths = db.list_paths_with_content_hash(hash)?;
        entry.insert(paths);
    }
    let path = unique_existing_path(
        context.rename_candidates_by_hash.get(hash),
        &context.existing,
    );
    Ok(path.and_then(|path| context.existing.remove(&path)))
}

fn take_rename_candidate_by_facts(
    db: &SourceDatabase,
    context: &mut ScanContext,
    size: u64,
    modified_ns: i64,
) -> Result<Option<WavEntry>, ScanError> {
    let key = (size, modified_ns);
    if let std::collections::hash_map::Entry::Vacant(entry) =
        context.rename_candidates_by_facts.entry(key)
    {
        let paths = db.list_paths_with_file_facts(size, modified_ns)?;
        entry.insert(paths);
    }
    let path = unique_existing_path(
        context.rename_candidates_by_facts.get(&key),
        &context.existing,
    );
    Ok(path.and_then(|path| context.existing.remove(&path)))
}

fn unique_existing_path(
    candidates: Option<&Vec<PathBuf>>,
    existing: &HashMap<PathBuf, WavEntry>,
) -> Option<PathBuf> {
    let candidates = candidates?;
    let mut match_path: Option<PathBuf> = None;
    for path in candidates {
        if !existing.contains_key(path) {
            continue;
        }
        if match_path.is_some() {
            return None;
        }
        match_path = Some(path.clone());
    }
    match_path
}

fn should_compute_full_hash(mode: ScanMode, size: u64) -> bool {
    match mode {
        ScanMode::Quick => size <= QUICK_HASH_MAX_BYTES,
        ScanMode::Hard => true,
    }
}

#[cfg(test)]
mod tests {
    use super::unique_existing_path;
    use crate::sample_sources::{Rating, WavEntry};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn entry(path: &str) -> WavEntry {
        WavEntry {
            relative_path: PathBuf::from(path),
            file_size: 1,
            modified_ns: 1,
            content_hash: None,
            tag: Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        }
    }

    #[test]
    fn unique_existing_path_returns_single_match() {
        let mut existing = HashMap::new();
        existing.insert(PathBuf::from("one.wav"), entry("one.wav"));
        existing.insert(PathBuf::from("two.wav"), entry("two.wav"));
        let candidates = vec![PathBuf::from("one.wav"), PathBuf::from("missing.wav")];

        let matched = unique_existing_path(Some(&candidates), &existing);

        assert_eq!(matched, Some(PathBuf::from("one.wav")));
    }

    #[test]
    fn unique_existing_path_rejects_ambiguous_matches() {
        let mut existing = HashMap::new();
        existing.insert(PathBuf::from("one.wav"), entry("one.wav"));
        existing.insert(PathBuf::from("two.wav"), entry("two.wav"));
        let candidates = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];

        let matched = unique_existing_path(Some(&candidates), &existing);

        assert_eq!(matched, None);
    }
}
