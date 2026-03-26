//! Controller-facing application of folder-delete recovery results.
//!
//! This keeps UI/cache updates separate from the filesystem recovery engine so startup
//! reconciliation stays testable without dragging in controller state mutation details.

use super::{
    DeleteRecoveryAction, DeleteRecoveryEntry, DeleteRecoveryReport, DeleteRecoveryStatus,
    DeleteStagingInfo, RetainedDeleteEntry, purge_deleted_folder, recover_staged_deletes,
    restore_deleted_folder,
};
use crate::app::controller::AppController;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::library::source_cache_invalidator;
use crate::app::state::{
    FolderActionPrompt, FolderDeleteRecoveryAction as UiDeleteRecoveryAction,
    FolderDeleteRecoveryEntry as UiDeleteRecoveryEntry,
    FolderDeleteRecoveryStatus as UiDeleteRecoveryStatus,
    RetainedFolderDeleteEntry as UiRetainedFolderDeleteEntry,
};
use crate::app::view_model;
use crate::sample_sources::{SampleSource, SourceId};
use std::collections::HashSet;
use tracing::warn;

impl AppController {
    /// Start background recovery for staged folder deletes after the UI is ready.
    pub(crate) fn start_folder_delete_recovery_if_needed(&mut self) {
        if self.runtime.delete_recovery_started || self.library.sources.is_empty() {
            return;
        }
        self.runtime.delete_recovery_started = true;
        self.ui.sources.folders.delete_recovery.in_progress = true;
        self.ui.sources.folders.delete_recovery.entries.clear();
        self.ui
            .sources
            .folders
            .delete_recovery
            .retained_entries
            .clear();
        let sources = self.library.sources.clone();
        let tx = self.runtime.jobs.message_sender();
        std::thread::spawn(move || {
            let report = recover_staged_deletes(&sources);
            let _ = tx.send(JobMessage::FolderDeleteRecoveryFinished(report));
        });
    }

    /// Apply staged delete recovery results to UI state and cached data.
    pub(crate) fn apply_folder_delete_recovery_report(&mut self, report: DeleteRecoveryReport) {
        self.ui.sources.folders.delete_recovery.in_progress = false;
        let (summary, errors) = RecoverySummary::from_report(self, report);
        let status_message = summary.status_message(errors.len());
        self.ui.sources.folders.delete_recovery.entries = summary.ui_entries;
        self.ui.sources.folders.delete_recovery.retained_entries = summary.retained_entries;
        if let Some((message, tone)) = status_message {
            self.set_status(message, tone);
        }
        for error in &errors {
            warn!(error = %error, "Delete recovery error");
        }
        self.refresh_recovered_sources(&summary.affected_sources);
    }

    /// Clear the staged delete recovery log.
    pub(crate) fn clear_folder_delete_recovery_log(&mut self) {
        self.ui.sources.folders.delete_recovery.entries.clear();
    }

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

    fn refresh_recovered_sources(&mut self, affected_sources: &HashSet<SourceId>) {
        if affected_sources.is_empty() {
            return;
        }
        let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
            &mut self.cache,
            &mut self.ui_cache,
            &mut self.library.missing,
        );
        for source_id in affected_sources {
            invalidator.invalidate_all(source_id);
        }
        if let Some(source) = self.current_source()
            && affected_sources.contains(&source.id)
        {
            if let Some(loaded) = self.sample_view.wav.loaded_wav.as_ref() {
                let absolute = source.root.join(loaded);
                if !absolute.is_file() {
                    self.clear_waveform_view();
                }
            }
            self.refresh_folder_browser();
            self.queue_wav_load();
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
        let failure_count = failures.len();
        if failure_count > 0 {
            for error in &failures {
                warn!(error = %error, "Retained folder delete resolution error");
            }
        }
        let action_label = resolution.label();
        let message = if failure_count == 0 {
            format!("{action_label} {resolved} retained folder delete(s)")
        } else {
            format!(
                "{action_label} {resolved} retained folder delete(s) ({} error(s))",
                failure_count
            )
        };
        self.set_status(
            message,
            if failure_count == 0 {
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
        restore_deleted_folder(&staged, &absolute, &staging_root)?;
        if entry.deleted_entries.is_empty() {
            scan_sources.insert(source.id.clone());
        } else {
            self.restore_folder_entries_in_db(&source, &entry.deleted_entries)?;
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

    fn refresh_folder_delete_recovery_state(&mut self) {
        let report = recover_staged_deletes(&self.library.sources.clone());
        let (summary, errors) = RecoverySummary::from_report(self, report);
        self.ui.sources.folders.delete_recovery.in_progress = false;
        self.ui.sources.folders.delete_recovery.entries = summary.ui_entries;
        self.ui.sources.folders.delete_recovery.retained_entries = summary.retained_entries;
        for error in &errors {
            warn!(error = %error, "Delete recovery error");
        }
    }
}

#[derive(Clone, Copy)]
enum RetainedDeleteResolution {
    Restore,
    Purge,
}

impl RetainedDeleteResolution {
    fn label(self) -> &'static str {
        match self {
            Self::Restore => "Restored",
            Self::Purge => "Purged",
        }
    }
}

#[derive(Default)]
struct RecoverySummary {
    ui_entries: Vec<UiDeleteRecoveryEntry>,
    retained_entries: Vec<UiRetainedFolderDeleteEntry>,
    affected_sources: HashSet<SourceId>,
    restored: usize,
    finalized: usize,
    failed: usize,
    retained: usize,
}

impl RecoverySummary {
    fn from_report(
        controller: &AppController,
        report: DeleteRecoveryReport,
    ) -> (Self, Vec<String>) {
        let mut summary = Self::default();
        for entry in report.entries {
            summary.push(controller, entry);
        }
        for entry in report.retained_entries {
            summary.push_retained(controller, entry);
        }
        (summary, report.errors)
    }

    fn push(&mut self, controller: &AppController, entry: DeleteRecoveryEntry) {
        let source_label = controller
            .library
            .sources
            .iter()
            .find(|source| source.id == entry.source_id)
            .map(|source| view_model::source_row(source, false).name)
            .unwrap_or_else(|| entry.source_root.to_string_lossy().to_string());
        let (action, status) = self.record_outcome(&entry);
        self.ui_entries.push(UiDeleteRecoveryEntry {
            source_label,
            relative_path: entry.original_relative,
            action,
            status,
            detail: entry.detail,
        });
    }

    fn push_retained(&mut self, controller: &AppController, entry: RetainedDeleteEntry) {
        let source_label = source_label(controller, &entry.source_id, &entry.source_root);
        self.retained += 1;
        self.retained_entries.push(UiRetainedFolderDeleteEntry {
            id: entry.id,
            source_id: entry.source_id,
            source_root: entry.source_root,
            source_label,
            relative_path: entry.original_relative,
            staged_relative: entry.staged_relative,
            deleted_entries: entry.deleted_entries,
        });
    }

    fn record_outcome(
        &mut self,
        entry: &DeleteRecoveryEntry,
    ) -> (UiDeleteRecoveryAction, UiDeleteRecoveryStatus) {
        match (entry.action, entry.status) {
            (DeleteRecoveryAction::Restore, DeleteRecoveryStatus::Completed) => {
                self.restored += 1;
                self.affected_sources.insert(entry.source_id.clone());
                (
                    UiDeleteRecoveryAction::Restore,
                    UiDeleteRecoveryStatus::Completed,
                )
            }
            (DeleteRecoveryAction::Finalize, DeleteRecoveryStatus::Completed) => {
                self.finalized += 1;
                self.affected_sources.insert(entry.source_id.clone());
                (
                    UiDeleteRecoveryAction::Finalize,
                    UiDeleteRecoveryStatus::Completed,
                )
            }
            (DeleteRecoveryAction::Restore, DeleteRecoveryStatus::Failed) => {
                self.failed += 1;
                (
                    UiDeleteRecoveryAction::Restore,
                    UiDeleteRecoveryStatus::Failed,
                )
            }
            (DeleteRecoveryAction::Finalize, DeleteRecoveryStatus::Failed) => {
                self.failed += 1;
                (
                    UiDeleteRecoveryAction::Finalize,
                    UiDeleteRecoveryStatus::Failed,
                )
            }
        }
    }

    fn status_message(&self, error_count: usize) -> Option<(String, StatusTone)> {
        let total = self.restored + self.finalized + self.failed;
        if total == 0 && self.retained == 0 && error_count == 0 {
            return None;
        }
        if total == 0 && self.retained == 0 {
            return Some((
                format!("Delete recovery encountered {error_count} error(s)"),
                StatusTone::Warning,
            ));
        }
        if total == 0 {
            let mut message = format!(
                "Recovery retained {} folder delete(s) for explicit restore or purge",
                self.retained
            );
            if error_count > 0 {
                message.push_str(&format!(" ({error_count} error(s))"));
            }
            return Some((
                message,
                if error_count > 0 {
                    StatusTone::Warning
                } else {
                    StatusTone::Info
                },
            ));
        }
        let mut message = format!(
            "Recovered {total} staged delete(s): {} restored, {} finalized",
            self.restored, self.finalized
        );
        if self.retained > 0 {
            message.push_str(&format!("; {} retained pending", self.retained));
        }
        if self.failed > 0 || error_count > 0 {
            message.push_str(&format!(" ({} error(s))", self.failed + error_count));
        }
        let tone = if self.failed > 0 || error_count > 0 {
            StatusTone::Warning
        } else {
            StatusTone::Info
        };
        Some((message, tone))
    }
}

fn source_label(
    controller: &AppController,
    source_id: &SourceId,
    source_root: &std::path::Path,
) -> String {
    controller
        .library
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .map(|source| view_model::source_row(source, false).name)
        .unwrap_or_else(|| source_root.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::dummy_controller;
    use crate::sample_sources::SampleSource;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn applying_recovery_report_updates_ui_entries() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.ui.sources.folders.delete_recovery.in_progress = true;
        let report = DeleteRecoveryReport {
            entries: vec![DeleteRecoveryEntry {
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                original_relative: "gone".into(),
                action: DeleteRecoveryAction::Restore,
                status: DeleteRecoveryStatus::Completed,
                detail: Some("Already restored".into()),
            }],
            retained_entries: Vec::new(),
            errors: Vec::new(),
        };

        controller.apply_folder_delete_recovery_report(report);

        assert!(!controller.ui.sources.folders.delete_recovery.in_progress);
        assert_eq!(
            controller.ui.sources.folders.delete_recovery.entries.len(),
            1
        );
        let entry = &controller.ui.sources.folders.delete_recovery.entries[0];
        assert_eq!(entry.source_label, "source");
        assert_eq!(entry.detail.as_deref(), Some("Already restored"));
    }

    #[test]
    fn clear_folder_delete_recovery_log_removes_entries() {
        let (mut controller, source) = dummy_controller();
        controller
            .ui
            .sources
            .folders
            .delete_recovery
            .entries
            .push(UiDeleteRecoveryEntry {
                source_label: source.root.to_string_lossy().to_string(),
                relative_path: "gone".into(),
                action: UiDeleteRecoveryAction::Restore,
                status: UiDeleteRecoveryStatus::Completed,
                detail: None,
            });

        controller.clear_folder_delete_recovery_log();

        assert!(
            controller
                .ui
                .sources
                .folders
                .delete_recovery
                .entries
                .is_empty()
        );
    }

    #[test]
    fn applying_recovery_uses_source_name_when_source_is_still_loaded() {
        let (mut controller, source) = named_source_controller("Drums");
        controller.ui.sources.folders.delete_recovery.in_progress = true;

        controller.apply_folder_delete_recovery_report(DeleteRecoveryReport {
            entries: vec![DeleteRecoveryEntry {
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                original_relative: "gone".into(),
                action: DeleteRecoveryAction::Finalize,
                status: DeleteRecoveryStatus::Completed,
                detail: None,
            }],
            retained_entries: Vec::new(),
            errors: Vec::new(),
        });

        assert_eq!(
            controller
                .ui
                .sources
                .folders
                .delete_recovery
                .retained_entries
                .len(),
            0
        );

        assert_eq!(
            controller.ui.sources.folders.delete_recovery.entries[0].source_label,
            "Drums"
        );
    }

    #[test]
    fn applying_recovery_report_tracks_retained_delete_entries() {
        let (mut controller, source) = named_source_controller("Drums");
        let deleted_entries = vec![crate::sample_sources::WavEntry {
            relative_path: "gone/kick.wav".into(),
            file_size: 42,
            modified_ns: 9,
            content_hash: Some("hash".into()),
            tag: crate::sample_sources::Rating::KEEP_3,
            looped: true,
            locked: true,
            missing: false,
            last_played_at: Some(12),
        }];

        controller.apply_folder_delete_recovery_report(DeleteRecoveryReport {
            entries: Vec::new(),
            retained_entries: vec![RetainedDeleteEntry {
                id: "retained-1".into(),
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                original_relative: "gone".into(),
                staged_relative: "gone".into(),
                deleted_entries: deleted_entries.clone(),
            }],
            errors: Vec::new(),
        });

        assert_eq!(
            controller
                .ui
                .sources
                .folders
                .delete_recovery
                .retained_entries
                .len(),
            1
        );
        let entry = &controller
            .ui
            .sources
            .folders
            .delete_recovery
            .retained_entries[0];
        assert_eq!(entry.source_label, "Drums");
        assert_eq!(entry.relative_path, std::path::Path::new("gone"));
        assert_eq!(entry.deleted_entries.len(), 1);
        assert_eq!(
            entry.deleted_entries[0].relative_path,
            deleted_entries[0].relative_path
        );
        assert_eq!(
            entry.deleted_entries[0].content_hash.as_deref(),
            deleted_entries[0].content_hash.as_deref()
        );
        assert_eq!(
            entry.deleted_entries[0].tag.val(),
            deleted_entries[0].tag.val()
        );
        assert_eq!(entry.deleted_entries[0].looped, deleted_entries[0].looped);
        assert_eq!(entry.deleted_entries[0].locked, deleted_entries[0].locked);
        assert_eq!(
            entry.deleted_entries[0].last_played_at,
            deleted_entries[0].last_played_at
        );
    }

    fn named_source_controller(name: &str) -> (AppController, SampleSource) {
        let (mut controller, source) = dummy_controller();
        let dir = tempdir().unwrap();
        let root = dir.path().join(name);
        fs::create_dir_all(&root).unwrap();
        std::mem::forget(dir);
        let source = SampleSource { root, ..source };
        controller.library.sources.push(source.clone());
        (controller, source)
    }
}
