//! Explicit restore and purge flows for retained folder deletes.

use super::restore_merge::{
    RestoredFileDisposition, RetainedRestoreMergeReport, restore_retained_folder_with_merge,
};
use super::{DeleteStagingInfo, purge_deleted_folder};
use crate::app::controller::AppController;
use crate::app::controller::StatusTone;
use crate::app::state::{
    FolderActionPrompt, RetainedFolderDeleteEntry as UiRetainedFolderDeleteEntry,
};
use crate::sample_sources::{SampleSource, SourceId, WavEntry};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::warn;

impl AppController {
    /// Open the explicit restore flow for retained folder deletes.
    pub(crate) fn start_restore_retained_folder_deletes(&mut self) {
        let entry_count = self
            .ui
            .sources
            .folders
            .delete_recovery
            .retained_entries
            .len();
        if entry_count == 0 {
            self.set_status("No retained folder deletes to restore", StatusTone::Info);
            return;
        }
        self.ui.sources.folders.pending_action =
            Some(FolderActionPrompt::RestoreRetainedDeletes { entry_count });
    }

    /// Open the explicit purge flow for retained folder deletes.
    pub(crate) fn start_purge_retained_folder_deletes(&mut self) {
        let entry_count = self
            .ui
            .sources
            .folders
            .delete_recovery
            .retained_entries
            .len();
        if entry_count == 0 {
            self.set_status("No retained folder deletes to purge", StatusTone::Info);
            return;
        }
        self.ui.sources.folders.pending_action =
            Some(FolderActionPrompt::PurgeRetainedDeletes { entry_count });
    }

    /// Apply the active retained-delete recovery prompt when present.
    pub(crate) fn apply_pending_folder_delete_recovery_prompt(&mut self) -> bool {
        let action = self.ui.sources.folders.pending_action.clone();
        let Some(action) = action else {
            return false;
        };
        let result = match action {
            FolderActionPrompt::RestoreRetainedDeletes { .. } => {
                self.resolve_retained_folder_deletes(RetainedDeleteResolution::Restore)
            }
            FolderActionPrompt::PurgeRetainedDeletes { .. } => {
                self.resolve_retained_folder_deletes(RetainedDeleteResolution::Purge)
            }
            FolderActionPrompt::Rename { .. } => return false,
        };
        self.ui.sources.folders.pending_action = None;
        if let Err(err) = result {
            self.set_status(err, StatusTone::Error);
        }
        true
    }

    /// Cancel the active retained-delete recovery prompt when present.
    pub(crate) fn cancel_folder_delete_recovery_prompt(&mut self) {
        if matches!(
            self.ui.sources.folders.pending_action,
            Some(FolderActionPrompt::RestoreRetainedDeletes { .. })
                | Some(FolderActionPrompt::PurgeRetainedDeletes { .. })
        ) {
            self.ui.sources.folders.pending_action = None;
        }
    }

    fn resolve_retained_folder_deletes(
        &mut self,
        resolution: RetainedDeleteResolution,
    ) -> Result<(), String> {
        let retained_entries = self
            .ui
            .sources
            .folders
            .delete_recovery
            .retained_entries
            .clone();
        if retained_entries.is_empty() {
            return Ok(());
        }
        let mut affected_sources = HashSet::new();
        let mut scan_sources = HashSet::new();
        let mut resolved = 0usize;
        let mut failures = Vec::new();
        for entry in &retained_entries {
            let result = match resolution {
                RetainedDeleteResolution::Restore => {
                    self.restore_retained_folder_delete(entry, &mut scan_sources)
                }
                RetainedDeleteResolution::Purge => self.purge_retained_folder_delete(entry),
            };
            match result {
                Ok(source_id) => {
                    affected_sources.insert(source_id);
                    resolved += 1;
                }
                Err(err) => failures.push(format!(
                    "{} ({}): {err}",
                    entry.source_label,
                    entry.relative_path.display()
                )),
            }
        }
        for source_id in &scan_sources {
            self.request_hard_sync_for_source(source_id);
        }
        self.refresh_folder_delete_recovery_state();
        self.refresh_recovered_sources(&affected_sources);
        if !failures.is_empty() {
            for error in &failures {
                warn!(error = %error, "Retained folder delete resolution error");
            }
        }
        self.set_status(
            status_message(resolution, resolved, failures.len()),
            if failures.is_empty() {
                StatusTone::Info
            } else {
                StatusTone::Warning
            },
        );
        Ok(())
    }

    fn restore_retained_folder_delete(
        &mut self,
        entry: &UiRetainedFolderDeleteEntry,
        scan_sources: &mut HashSet<SourceId>,
    ) -> Result<SourceId, String> {
        let source = self.retained_delete_source(entry);
        let staging_root = source.root.join(super::DELETE_STAGING_DIR);
        let absolute = source.root.join(&entry.relative_path);
        let staged = DeleteStagingInfo {
            id: entry.id.clone(),
            original_relative: entry.relative_path.clone(),
            staged_relative: entry.staged_relative.clone(),
            staged_absolute: staging_root.join(&entry.staged_relative),
        };
        let existing_entries =
            self.snapshot_existing_restore_entries(&source, &entry.deleted_entries)?;
        let merge =
            restore_retained_folder_with_merge(&staged, &source.root, &absolute, &staging_root)?;
        self.apply_retained_restore_db_entries(
            &source,
            &entry.deleted_entries,
            &existing_entries,
            &merge,
        )?;
        if entry.deleted_entries.is_empty() || merge.had_conflicts {
            scan_sources.insert(source.id.clone());
        }
        Ok(source.id)
    }

    fn purge_retained_folder_delete(
        &mut self,
        entry: &UiRetainedFolderDeleteEntry,
    ) -> Result<SourceId, String> {
        let source = self.retained_delete_source(entry);
        let staging_root = source.root.join(super::DELETE_STAGING_DIR);
        let staged = DeleteStagingInfo {
            id: entry.id.clone(),
            original_relative: entry.relative_path.clone(),
            staged_relative: entry.staged_relative.clone(),
            staged_absolute: staging_root.join(&entry.staged_relative),
        };
        purge_deleted_folder(&staged, &staging_root)?;
        Ok(source.id)
    }

    fn retained_delete_source(&self, entry: &UiRetainedFolderDeleteEntry) -> SampleSource {
        self.library
            .sources
            .iter()
            .find(|source| source.id == entry.source_id)
            .cloned()
            .unwrap_or_else(|| {
                SampleSource::new_with_id(entry.source_id.clone(), entry.source_root.clone())
            })
    }

    fn snapshot_existing_restore_entries(
        &self,
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
        &mut self,
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
        self.restore_folder_entries_in_db(source, &restore_rows)
    }
}

#[derive(Clone, Copy)]
enum RetainedDeleteResolution {
    Restore,
    Purge,
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

fn status_message(
    resolution: RetainedDeleteResolution,
    resolved: usize,
    failures: usize,
) -> String {
    let label = match resolution {
        RetainedDeleteResolution::Restore => "Restored",
        RetainedDeleteResolution::Purge => "Purged",
    };
    if failures == 0 {
        return format!("{label} {resolved} retained folder delete(s)");
    }
    format!("{label} {resolved} retained folder delete(s) ({failures} error(s))")
}

#[cfg(test)]
mod tests;
