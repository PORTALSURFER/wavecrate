//! Background retained-delete resolution helpers.
//!
//! Explicit restore/purge must not block the UI thread. This module owns the
//! worker-side filesystem and database reconciliation used by the shared
//! `FileOps` pipeline.

use super::restore_merge::{new_restore_stamp, restore_retained_folder_with_merge_with_stamp};
use super::retained_restore_reconcile::{
    apply_retained_restore_db_entries, snapshot_existing_restore_entries,
};
use super::{
    DELETE_STAGING_DIR, DeleteStagingInfo, mark_delete_restore_pending_db, purge_deleted_folder,
    recover_staged_deletes, remove_delete_entry,
};
use crate::app::controller::jobs::{
    FileOpMessage, RetainedDeleteResolutionEntry, RetainedDeleteResolutionRequest,
    RetainedDeleteResolutionResult, RetainedDeleteResolutionSource,
};
use crate::sample_sources::{SampleSource, SourceId};
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
    let stamp = new_restore_stamp()?;
    mark_delete_restore_pending_db(&staging_root, &staged.id, &stamp)?;
    let merge = restore_retained_folder_with_merge_with_stamp(
        &staged,
        &source.root,
        &absolute,
        &staging_root,
        &stamp,
    )?;
    apply_retained_restore_db_entries(&source, &entry.deleted_entries, &existing_entries, &merge)?;
    remove_delete_entry(&staging_root, &staged.id)?;
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
