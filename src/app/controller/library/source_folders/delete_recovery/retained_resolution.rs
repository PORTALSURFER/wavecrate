//! Background retained-delete resolution helpers.
//!
//! Explicit restore/purge must not block the UI thread. This module owns the
//! worker-side filesystem and database reconciliation used by the shared
//! `FileOps` pipeline.

use super::restore_merge::{
    RestoredFileDisposition, RetainedRestoreMergeReport, restore_retained_folder_with_merge,
};
use super::{DELETE_STAGING_DIR, DeleteStagingInfo, purge_deleted_folder, recover_staged_deletes};
use crate::app::controller::jobs::{
    FileOpMessage, RetainedDeleteResolutionEntry, RetainedDeleteResolutionRequest,
    RetainedDeleteResolutionResult, RetainedDeleteResolutionSource,
};
use crate::sample_sources::{SampleSource, SourceDatabase, SourceId, WavEntry};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Run explicit retained-delete restore/purge work in the background.
pub(crate) fn run_retained_delete_resolution_job(
    request: RetainedDeleteResolutionRequest,
    tx: Option<&Sender<FileOpMessage>>,
) -> RetainedDeleteResolutionResult {
    let mut resolved = 0usize;
    let mut affected_sources = Vec::new();
    let mut scan_sources = Vec::new();
    let mut failures = Vec::new();

    for (index, entry) in request.entries.iter().enumerate() {
        let result = match request.mode {
            crate::app::controller::jobs::RetainedDeleteResolutionMode::Restore => {
                restore_retained_entry(entry)
            }
            crate::app::controller::jobs::RetainedDeleteResolutionMode::Purge => {
                purge_retained_entry(entry)
            }
        };
        match result {
            Ok(outcome) => {
                resolved += 1;
                push_unique_source(&mut affected_sources, outcome.source_id.clone());
                if outcome.needs_hard_sync {
                    push_unique_source(&mut scan_sources, outcome.source_id);
                }
            }
            Err(err) => failures.push(format!(
                "{} ({}): {err}",
                entry.source_label,
                entry.relative_path.display()
            )),
        }
        if let Some(tx) = tx {
            let _ = tx.send(FileOpMessage::Progress {
                completed: index + 1,
                detail: Some(format!(
                    "{} {}: {}",
                    request.mode.status_label(),
                    entry.source_label,
                    entry.relative_path.display()
                )),
            });
        }
    }

    let sources = build_sources(&request.sources);
    let recovery_report = recover_staged_deletes(&sources);

    RetainedDeleteResolutionResult {
        mode: request.mode,
        resolved,
        affected_sources,
        scan_sources,
        failures,
        recovery_report,
    }
}

struct EntryResolutionOutcome {
    source_id: SourceId,
    needs_hard_sync: bool,
}

fn restore_retained_entry(
    entry: &RetainedDeleteResolutionEntry,
) -> Result<EntryResolutionOutcome, String> {
    let source = SampleSource::new_with_id(entry.source_id.clone(), entry.source_root.clone());
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let absolute = source.root.join(&entry.relative_path);
    let staged = DeleteStagingInfo {
        id: entry.id.clone(),
        original_relative: entry.relative_path.clone(),
        staged_relative: entry.staged_relative.clone(),
        staged_absolute: staging_root.join(&entry.staged_relative),
    };
    let existing_entries = snapshot_existing_restore_entries(&source, &entry.deleted_entries)?;
    let merge =
        restore_retained_folder_with_merge(&staged, &source.root, &absolute, &staging_root)?;
    apply_retained_restore_db_entries(&source, &entry.deleted_entries, &existing_entries, &merge)?;
    Ok(EntryResolutionOutcome {
        source_id: source.id,
        needs_hard_sync: entry.deleted_entries.is_empty() || merge.had_conflicts,
    })
}

fn purge_retained_entry(
    entry: &RetainedDeleteResolutionEntry,
) -> Result<EntryResolutionOutcome, String> {
    let source = SampleSource::new_with_id(entry.source_id.clone(), entry.source_root.clone());
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = DeleteStagingInfo {
        id: entry.id.clone(),
        original_relative: entry.relative_path.clone(),
        staged_relative: entry.staged_relative.clone(),
        staged_absolute: staging_root.join(&entry.staged_relative),
    };
    purge_deleted_folder(&staged, &staging_root)?;
    Ok(EntryResolutionOutcome {
        source_id: source.id,
        needs_hard_sync: false,
    })
}

fn snapshot_existing_restore_entries(
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

fn apply_retained_restore_db_entries(
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

fn build_sources(sources: &[RetainedDeleteResolutionSource]) -> Vec<SampleSource> {
    sources
        .iter()
        .map(|source| {
            SampleSource::new_with_id(source.source_id.clone(), source.source_root.clone())
        })
        .collect()
}

fn push_unique_source(target: &mut Vec<SourceId>, source_id: SourceId) {
    if !target.iter().any(|existing| existing == &source_id) {
        target.push(source_id);
    }
}
