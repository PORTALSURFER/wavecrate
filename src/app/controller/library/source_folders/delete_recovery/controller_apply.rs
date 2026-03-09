//! Controller-facing application of folder-delete recovery results.
//!
//! This keeps UI/cache updates separate from the filesystem recovery engine so startup
//! reconciliation stays testable without dragging in controller state mutation details.

use super::{
    DeleteRecoveryAction, DeleteRecoveryEntry, DeleteRecoveryReport, DeleteRecoveryStatus,
    recover_staged_deletes,
};
use crate::app::controller::AppController;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::library::source_cache_invalidator;
use crate::app::state::{
    FolderDeleteRecoveryAction as UiDeleteRecoveryAction,
    FolderDeleteRecoveryEntry as UiDeleteRecoveryEntry,
    FolderDeleteRecoveryStatus as UiDeleteRecoveryStatus,
};
use crate::app::view_model;
use crate::sample_sources::SourceId;
use std::collections::HashSet;

impl AppController {
    /// Start background recovery for staged folder deletes after the UI is ready.
    pub(crate) fn start_folder_delete_recovery_if_needed(&mut self) {
        if self.runtime.delete_recovery_started || self.library.sources.is_empty() {
            return;
        }
        self.runtime.delete_recovery_started = true;
        self.ui.sources.folders.delete_recovery.in_progress = true;
        self.ui.sources.folders.delete_recovery.entries.clear();
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
        if let Some((message, tone)) = status_message {
            self.set_status(message, tone);
        }
        for error in &errors {
            eprintln!("Delete recovery error: {error}");
        }
        self.refresh_recovered_sources(&summary.affected_sources);
    }

    /// Clear the staged delete recovery log.
    pub(crate) fn clear_folder_delete_recovery_log(&mut self) {
        self.ui.sources.folders.delete_recovery.entries.clear();
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
}

#[derive(Default)]
struct RecoverySummary {
    ui_entries: Vec<UiDeleteRecoveryEntry>,
    affected_sources: HashSet<SourceId>,
    restored: usize,
    finalized: usize,
    failed: usize,
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
        if total == 0 && error_count == 0 {
            return None;
        }
        if total == 0 {
            return Some((
                format!("Delete recovery encountered {error_count} error(s)"),
                StatusTone::Warning,
            ));
        }
        let mut message = format!(
            "Recovered {total} staged delete(s): {} restored, {} finalized",
            self.restored, self.finalized
        );
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
            errors: Vec::new(),
        });

        assert_eq!(
            controller.ui.sources.folders.delete_recovery.entries[0].source_label,
            "Drums"
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
