use std::path::{Path, PathBuf};

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::{SourceWriteBatch, WavEntry};

use super::scan::{
    ChangedSample, RenamedSample, ScanContext, ScanError, ScanMode, ScanStats, UpdatedSample,
};
use super::scan_fs::{FileFacts, is_supported_scannable_audio_file};

const QUICK_HASH_MAX_BYTES: u64 = 8 * 1024 * 1024;

pub(super) struct PreparedFile {
    pub(super) facts: FileFacts,
    pub(super) hash_required: bool,
    pub(super) needs_hash: bool,
    pub(super) requires_apply: bool,
    pub(super) content_hash: Option<String>,
}

pub(super) fn apply_diff(
    db: &SourceDatabase,
    batch: &mut SourceWriteBatch<'_>,
    prepared: PreparedFile,
    context: &mut ScanContext,
    root: &Path,
) -> Result<(), ScanError> {
    let PreparedFile {
        facts,
        hash_required: _,
        needs_hash: _,
        requires_apply: _,
        content_hash,
    } = prepared;
    let path = facts.relative.clone();
    let should_hash = should_compute_full_hash(context.mode, facts.size);
    let _ = context.existing.remove(&path);
    let existing = db.entry_for_path(&path)?;
    match existing {
        Some(entry) if entry.file_size == facts.size && entry.modified_ns == facts.modified_ns => {
            if entry.missing {
                batch.set_missing(&path, false)?;
            }
            if entry.content_hash.is_none() {
                if should_hash {
                    let hash = required_prepared_hash(content_hash);
                    batch.upsert_file_with_hash(&path, facts.size, facts.modified_ns, &hash)?;
                    context.stats.hashes_computed += 1;
                } else {
                    context.stats.hashes_pending += 1;
                }
            }
        }
        Some(entry) => {
            let previous_hash = entry.content_hash.as_deref();
            if should_hash {
                let hash = required_prepared_hash(content_hash);
                batch.upsert_file_with_hash(&path, facts.size, facts.modified_ns, &hash)?;
                context.stats.hashes_computed += 1;
                context.stats.updated_samples.push(UpdatedSample {
                    relative_path: path.clone(),
                    file_size: facts.size,
                    modified_ns: facts.modified_ns,
                    content_hash: Some(hash.clone()),
                });
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
                context.stats.updated_samples.push(UpdatedSample {
                    relative_path: path.clone(),
                    file_size: facts.size,
                    modified_ns: facts.modified_ns,
                    content_hash: None,
                });
            }
            context.stats.updated += 1;
        }
        None => {
            if should_hash {
                let hash = required_prepared_hash(content_hash);
                if let Some(entry) = take_rename_candidate_by_hash(db, batch, context, root, &hash)?
                {
                    let old_relative_path = entry.relative_path.clone();
                    apply_rename(batch, &path, &facts, &hash, entry, None)?;
                    context.stats.updated += 1;
                    context.stats.renames_reconciled += 1;
                    context.stats.renamed_samples.push(RenamedSample {
                        old_relative_path,
                        new_relative_path: path.clone(),
                        file_size: facts.size,
                        modified_ns: facts.modified_ns,
                        content_hash: Some(hash),
                    });
                    return Ok(());
                }
                if let Some(entry) = batch.take_pending_rename_by_hash(&hash)? {
                    let normal_tags = entry.normal_tags.clone();
                    let entry = entry.into_wav_entry();
                    let old_relative_path = entry.relative_path.clone();
                    apply_rename(batch, &path, &facts, &hash, entry, Some(&normal_tags))?;
                    context.stats.updated += 1;
                    context.stats.renames_reconciled += 1;
                    context.stats.hashes_computed += 1;
                    context.stats.renamed_samples.push(RenamedSample {
                        old_relative_path,
                        new_relative_path: path.clone(),
                        file_size: facts.size,
                        modified_ns: facts.modified_ns,
                        content_hash: Some(hash),
                    });
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
                if let Some(entry) = take_rename_candidate_by_facts(
                    db,
                    batch,
                    context,
                    root,
                    facts.size,
                    facts.modified_ns,
                )? {
                    let old_relative_path = entry.relative_path.clone();
                    apply_rename_without_hash(batch, &path, &facts, entry, None)?;
                    context.stats.updated += 1;
                    context.stats.renames_reconciled += 1;
                    context.stats.hashes_pending += 1;
                    context.stats.renamed_samples.push(RenamedSample {
                        old_relative_path,
                        new_relative_path: path.clone(),
                        file_size: facts.size,
                        modified_ns: facts.modified_ns,
                        content_hash: None,
                    });
                    return Ok(());
                }
                if let Some(entry) =
                    batch.take_pending_rename_by_facts(facts.size, facts.modified_ns)?
                {
                    let normal_tags = entry.normal_tags.clone();
                    let entry = entry.into_wav_entry();
                    let old_relative_path = entry.relative_path.clone();
                    apply_rename_without_hash(batch, &path, &facts, entry, Some(&normal_tags))?;
                    context.stats.updated += 1;
                    context.stats.renames_reconciled += 1;
                    context.stats.hashes_pending += 1;
                    context.stats.renamed_samples.push(RenamedSample {
                        old_relative_path,
                        new_relative_path: path.clone(),
                        file_size: facts.size,
                        modified_ns: facts.modified_ns,
                        content_hash: None,
                    });
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

fn required_prepared_hash(content_hash: Option<String>) -> String {
    content_hash.expect("hash-required scan entries must be prepared before opening a write batch")
}

pub(super) fn mark_missing(
    db: &SourceDatabase,
    batch: &mut SourceWriteBatch<'_>,
    existing: impl IntoIterator<Item = WavEntry>,
    stats: &mut ScanStats,
    mode: ScanMode,
) -> Result<(), ScanError> {
    for stale in existing {
        if is_supported_scannable_audio_file(db.root(), &stale.relative_path) {
            continue;
        }
        let Some(leftover) = db.entry_for_path(&stale.relative_path)? else {
            continue;
        };
        if matches!(mode, ScanMode::Targeted | ScanMode::Quick) {
            batch.stage_pending_rename(&leftover)?;
        }
        batch.remove_file(&leftover.relative_path)?;
        stats.missing += 1;
    }
    Ok(())
}

fn apply_rename(
    batch: &mut SourceWriteBatch<'_>,
    new_path: &Path,
    facts: &FileFacts,
    hash: &str,
    entry: WavEntry,
    retained_normal_tags: Option<&[String]>,
) -> Result<(), ScanError> {
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
    if entry.sound_type.is_some() {
        batch.set_sound_type(new_path, entry.sound_type)?;
    }
    if entry.locked {
        batch.set_locked(new_path, entry.locked)?;
    }
    if let Some(last_played_at) = entry.last_played_at {
        batch.set_last_played_at(new_path, last_played_at)?;
    }
    if entry.user_tag.is_some() {
        batch.set_user_tag(new_path, entry.user_tag.as_deref())?;
    }
    batch.set_tag_named(new_path, entry.tag_named)?;
    if let Some(normal_tags) = retained_normal_tags {
        batch.replace_tags_for_path(new_path, normal_tags)?;
    } else {
        batch.copy_tags_between_paths(&entry.relative_path, new_path)?;
    }
    batch.remove_file(&entry.relative_path)?;
    batch.remap_analysis_sample_identity(&entry.relative_path, new_path)?;
    Ok(())
}

fn apply_rename_without_hash(
    batch: &mut SourceWriteBatch<'_>,
    new_path: &Path,
    facts: &FileFacts,
    entry: WavEntry,
    retained_normal_tags: Option<&[String]>,
) -> Result<(), ScanError> {
    batch.upsert_file_without_hash(new_path, facts.size, facts.modified_ns)?;
    batch.set_tag(new_path, entry.tag)?;
    if entry.looped {
        batch.set_looped(new_path, entry.looped)?;
    }
    if entry.sound_type.is_some() {
        batch.set_sound_type(new_path, entry.sound_type)?;
    }
    if entry.locked {
        batch.set_locked(new_path, entry.locked)?;
    }
    if let Some(last_played_at) = entry.last_played_at {
        batch.set_last_played_at(new_path, last_played_at)?;
    }
    if entry.user_tag.is_some() {
        batch.set_user_tag(new_path, entry.user_tag.as_deref())?;
    }
    batch.set_tag_named(new_path, entry.tag_named)?;
    if let Some(normal_tags) = retained_normal_tags {
        batch.replace_tags_for_path(new_path, normal_tags)?;
    } else {
        batch.copy_tags_between_paths(&entry.relative_path, new_path)?;
    }
    batch.remove_file(&entry.relative_path)?;
    batch.remap_analysis_sample_identity(&entry.relative_path, new_path)?;
    Ok(())
}

fn take_rename_candidate_by_hash(
    db: &SourceDatabase,
    batch: &mut SourceWriteBatch<'_>,
    context: &mut ScanContext,
    root: &Path,
    hash: &str,
) -> Result<Option<WavEntry>, ScanError> {
    let current_candidates =
        rename_candidates_in_scope(batch.list_paths_with_content_hash(hash)?, context);
    let path = unique_missing_path(&current_candidates, root);
    let Some(path) = path else {
        return Ok(None);
    };
    let Some(entry) = db.entry_for_path(&path)? else {
        return Ok(None);
    };
    if entry.content_hash.as_deref() != Some(hash) {
        return Ok(None);
    }
    let _ = context.existing.remove(&path);
    Ok(Some(entry))
}

fn take_rename_candidate_by_facts(
    db: &SourceDatabase,
    batch: &mut SourceWriteBatch<'_>,
    context: &mut ScanContext,
    root: &Path,
    size: u64,
    modified_ns: i64,
) -> Result<Option<WavEntry>, ScanError> {
    let current_candidates = rename_candidates_in_scope(
        batch.list_paths_with_file_facts(size, modified_ns)?,
        context,
    );
    let path = unique_missing_path(&current_candidates, root);
    let Some(path) = path else {
        return Ok(None);
    };
    let Some(entry) = db.entry_for_path(&path)? else {
        return Ok(None);
    };
    if entry.file_size != size || entry.modified_ns != modified_ns {
        return Ok(None);
    }
    let _ = context.existing.remove(&path);
    Ok(Some(entry))
}

fn rename_candidates_in_scope(mut candidates: Vec<PathBuf>, context: &ScanContext) -> Vec<PathBuf> {
    if context.mode == ScanMode::Targeted {
        candidates.retain(|path| context.existing.contains_key(path));
    }
    candidates
}

fn unique_missing_path(candidates: &[PathBuf], root: &Path) -> Option<PathBuf> {
    let mut match_path: Option<PathBuf> = None;
    for path in candidates {
        if root.join(path).exists() {
            continue;
        }
        if match_path.is_some() {
            return None;
        }
        match_path = Some(path.clone());
    }
    match_path
}

pub(super) fn should_compute_full_hash(mode: ScanMode, size: u64) -> bool {
    match mode {
        ScanMode::Targeted | ScanMode::Quick => size <= QUICK_HASH_MAX_BYTES,
        ScanMode::Hard => true,
    }
}

#[cfg(test)]
mod tests {
    use super::unique_missing_path;
    use std::path::PathBuf;

    #[test]
    fn unique_missing_path_returns_single_match() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("present.wav"), b"present").unwrap();
        let candidates = vec![PathBuf::from("missing.wav"), PathBuf::from("present.wav")];

        let matched = unique_missing_path(&candidates, root.path());

        assert_eq!(matched, Some(PathBuf::from("missing.wav")));
    }

    #[test]
    fn unique_missing_path_rejects_ambiguous_current_rows() {
        let root = tempfile::tempdir().unwrap();
        let candidates = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];

        let matched = unique_missing_path(&candidates, root.path());

        assert_eq!(matched, None);
    }

    #[test]
    fn unique_existing_path_rejects_candidate_recreated_before_claim() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("one.wav"), b"recreated").unwrap();
        let candidates = vec![PathBuf::from("one.wav")];

        let matched = unique_missing_path(&candidates, root.path());

        assert_eq!(matched, None);
    }
}
