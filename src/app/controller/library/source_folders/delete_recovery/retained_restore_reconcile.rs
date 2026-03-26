//! Shared retained-restore reconciliation helpers.
//!
//! Explicit retained restores can finish their filesystem merge before metadata replay
//! completes. These helpers keep the database-replay logic reusable from both the
//! live worker path and startup crash recovery.

use super::restore_merge::{
    ExistingFileRelocation, RestoredFileDisposition, RestoredFileRecord, RetainedRestoreMergeReport,
};
use crate::sample_sources::{SampleSource, SourceDatabase, WavEntry};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Snapshot existing metadata rows that could be preserved during retained restore.
pub(crate) fn snapshot_existing_restore_entries(
    source: &SampleSource,
    deleted_entries: &[WavEntry],
) -> Result<HashMap<PathBuf, WavEntry>, String> {
    if deleted_entries.is_empty() {
        return Ok(HashMap::new());
    }
    let db = source
        .open_db()
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let mut rows = HashMap::new();
    for entry in deleted_entries {
        if let Some(current) = db
            .entry_for_path(&entry.relative_path)
            .map_err(|err| format!("Failed to read existing restore metadata: {err}"))?
        {
            rows.insert(entry.relative_path.clone(), current);
        }
    }
    Ok(rows)
}

/// Apply the restored metadata rows for one retained restore outcome.
pub(crate) fn apply_retained_restore_db_entries(
    source: &SampleSource,
    deleted_entries: &[WavEntry],
    existing_entries: &HashMap<PathBuf, WavEntry>,
    merge: &RetainedRestoreMergeReport,
) -> Result<(), String> {
    if deleted_entries.is_empty() {
        return Ok(());
    }
    let mut restore_rows = relocated_existing_entries(existing_entries, merge);
    restore_rows.extend(restored_deleted_entries(
        deleted_entries,
        existing_entries,
        merge,
    )?);
    restore_rows_in_db(source, &restore_rows)
}

/// Infer the final merge report for a pending retained restore from the live filesystem.
pub(crate) fn infer_retained_restore_merge_report(
    source_root: &Path,
    deleted_entries: &[WavEntry],
    existing_entries: &HashMap<PathBuf, WavEntry>,
    stamp: &str,
) -> Result<RetainedRestoreMergeReport, String> {
    let mut report = RetainedRestoreMergeReport::default();
    for deleted in deleted_entries {
        let original = deleted.relative_path.clone();
        if let Some(replaced_relative) =
            find_timestamped_variant(source_root, &original, "replaced", stamp)?
        {
            report.had_conflicts = true;
            report.restored_files.push(RestoredFileRecord {
                original_relative: original.clone(),
                final_relative: original.clone(),
                disposition: RestoredFileDisposition::RestoredCanonical,
            });
            report.existing_relocations.push(ExistingFileRelocation {
                original_relative: original,
                relocated_relative: replaced_relative,
            });
            continue;
        }
        if let Some(recovered_relative) =
            recovered_relative_for_deleted_entry(source_root, &original, stamp)?
        {
            report.had_conflicts = true;
            report.restored_files.push(RestoredFileRecord {
                original_relative: original,
                final_relative: recovered_relative,
                disposition: RestoredFileDisposition::RestoredTimestamped,
            });
            continue;
        }
        if source_root.join(&deleted.relative_path).is_file() {
            let disposition = if existing_entries.contains_key(&deleted.relative_path) {
                RestoredFileDisposition::ReusedExisting
            } else {
                RestoredFileDisposition::RestoredCanonical
            };
            report.restored_files.push(RestoredFileRecord {
                original_relative: deleted.relative_path.clone(),
                final_relative: deleted.relative_path.clone(),
                disposition,
            });
            continue;
        }
        return Err(format!(
            "Missing retained restore result for {}",
            deleted.relative_path.display()
        ));
    }
    Ok(report)
}

fn relocated_existing_entries(
    existing_entries: &HashMap<PathBuf, WavEntry>,
    merge: &RetainedRestoreMergeReport,
) -> Vec<WavEntry> {
    let mut rows = Vec::new();
    for relocation in &merge.existing_relocations {
        if let Some(existing) = existing_entries.get(&relocation.original_relative) {
            let mut relocated = existing.clone();
            relocated.relative_path = relocation.relocated_relative.clone();
            rows.push(relocated);
        }
    }
    rows
}

fn restored_deleted_entries(
    deleted_entries: &[WavEntry],
    existing_entries: &HashMap<PathBuf, WavEntry>,
    merge: &RetainedRestoreMergeReport,
) -> Result<Vec<WavEntry>, String> {
    let mut rows = Vec::new();
    for deleted in deleted_entries {
        let record = merge
            .restored_record_for(&deleted.relative_path)
            .ok_or_else(|| {
                format!(
                    "Missing retained restore result for {}",
                    deleted.relative_path.display()
                )
            })?;
        if matches!(record.disposition, RestoredFileDisposition::ReusedExisting)
            && existing_entries.contains_key(&deleted.relative_path)
        {
            continue;
        }
        let mut restored = deleted.clone();
        restored.relative_path = record.final_relative.clone();
        rows.push(restored);
    }
    Ok(rows)
}

fn restore_rows_in_db(source: &SampleSource, entries: &[WavEntry]) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    let db =
        SourceDatabase::open(&source.root).map_err(|err| format!("Database unavailable: {err}"))?;
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to start database update: {err}"))?;
    for entry in entries {
        if let Some(content_hash) = entry.content_hash.as_deref() {
            batch
                .upsert_file_with_hash(
                    &entry.relative_path,
                    entry.file_size,
                    entry.modified_ns,
                    content_hash,
                )
                .map_err(|err| format!("Failed to restore database row: {err}"))?;
        } else {
            batch
                .upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
                .map_err(|err| format!("Failed to restore database row: {err}"))?;
        }
        batch
            .set_tag(&entry.relative_path, entry.tag)
            .map_err(|err| format!("Failed to restore tag: {err}"))?;
        batch
            .set_looped(&entry.relative_path, entry.looped)
            .map_err(|err| format!("Failed to restore loop marker: {err}"))?;
        batch
            .set_locked(&entry.relative_path, entry.locked)
            .map_err(|err| format!("Failed to restore keep lock: {err}"))?;
        if let Some(last_played_at) = entry.last_played_at {
            batch
                .set_last_played_at(&entry.relative_path, last_played_at)
                .map_err(|err| format!("Failed to restore playback age: {err}"))?;
        }
    }
    batch
        .commit()
        .map_err(|err| format!("Failed to restore folder delete state: {err}"))
}

fn recovered_relative_for_deleted_entry(
    source_root: &Path,
    original: &Path,
    stamp: &str,
) -> Result<Option<PathBuf>, String> {
    let mut ancestors = Vec::new();
    let mut current = original.parent();
    while let Some(dir) = current {
        if dir.as_os_str().is_empty() {
            break;
        }
        ancestors.push(dir.to_path_buf());
        current = dir.parent();
    }
    for ancestor in ancestors.iter().rev() {
        if let Some(recovered_dir) =
            find_timestamped_variant(source_root, ancestor, "recovered", stamp)?
        {
            let remainder = original.strip_prefix(ancestor).map_err(|_| {
                format!("Failed to resolve restored path for {}", original.display())
            })?;
            return Ok(Some(recovered_dir.join(remainder)));
        }
    }
    find_timestamped_variant(source_root, original, "recovered", stamp)
}

fn find_timestamped_variant(
    source_root: &Path,
    original_relative: &Path,
    label: &str,
    stamp: &str,
) -> Result<Option<PathBuf>, String> {
    let original_name = original_relative
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("Path missing file name: {}", original_relative.display()))?;
    let parent_relative = original_relative.parent().unwrap_or_else(|| Path::new(""));
    let parent = source_root.join(parent_relative);
    if !parent.is_dir() {
        return Ok(None);
    }
    let (base, extension) = split_name(original_name, original_relative.extension().is_some());
    let mut matches = Vec::new();
    for entry in std::fs::read_dir(&parent).map_err(|err| {
        format!(
            "Failed to inspect restore directory {}: {err}",
            parent.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "Failed to enumerate restore directory {}: {err}",
                parent.display()
            )
        })?;
        let name = entry.file_name();
        if !matches_timestamped_variant(name.as_os_str(), &base, extension.as_deref(), label, stamp)
        {
            continue;
        }
        matches.push(parent_relative.join(name));
    }
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.pop()),
        _ => Err(format!(
            "Ambiguous retained restore state for {} ({label}-{stamp})",
            original_relative.display()
        )),
    }
}

fn matches_timestamped_variant(
    candidate: &OsStr,
    base: &str,
    extension: Option<&str>,
    label: &str,
    stamp: &str,
) -> bool {
    let candidate = candidate.to_string_lossy();
    let prefix = format!("{base}.{label}-{stamp}");
    if let Some(ext) = extension {
        candidate == format!("{prefix}.{ext}")
            || candidate.starts_with(&format!("{prefix}-"))
                && candidate.ends_with(&format!(".{ext}"))
    } else {
        candidate == prefix || candidate.starts_with(&format!("{prefix}-"))
    }
}

fn split_name(name: &str, is_file: bool) -> (String, Option<String>) {
    if !is_file {
        return (name.to_string(), None);
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_string);
    (stem.to_string(), extension)
}
