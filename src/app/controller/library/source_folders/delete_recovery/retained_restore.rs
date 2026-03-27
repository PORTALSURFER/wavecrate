//! Explicit restore and purge flows for retained folder deletes.

use super::run_retained_delete_resolution_job;
use crate::app::controller::AppController;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, RetainedDeleteBusyEntry, RetainedDeleteResolutionEntry,
    RetainedDeleteResolutionMode, RetainedDeleteResolutionRequest, RetainedDeleteResolutionResult,
    RetainedDeleteResolutionSource,
};
use crate::app::state::{
    FolderActionPrompt, RetainedFolderDeleteEntry as UiRetainedFolderDeleteEntry,
};
use crate::sample_sources::SourceId;
use std::collections::HashSet;
use std::path::Path;
use tracing::warn;

#[cfg(not(test))]
use crate::app::controller::jobs::{FileOpMessage, FileOpResult};
#[cfg(not(test))]
use std::sync::{Arc, atomic::AtomicBool};

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
        let started = match action {
            FolderActionPrompt::RestoreRetainedDeletes { .. } => {
                self.begin_retained_delete_resolution(RetainedDeleteResolutionMode::Restore)
            }
            FolderActionPrompt::PurgeRetainedDeletes { .. } => {
                self.begin_retained_delete_resolution(RetainedDeleteResolutionMode::Purge)
            }
        };
        match started {
            Ok(true) => {
                self.ui.sources.folders.pending_action = None;
            }
            Ok(false) => {}
            Err(err) => {
                self.set_status(err, StatusTone::Error);
            }
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

    fn begin_retained_delete_resolution(
        &mut self,
        mode: RetainedDeleteResolutionMode,
    ) -> Result<bool, String> {
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status(
                "Another file operation is already running",
                StatusTone::Warning,
            );
            return Ok(false);
        }
        let request = self.retained_delete_resolution_request(mode);
        if request.entries.is_empty() {
            return Ok(false);
        }
        self.runtime.active_retained_delete_resolution =
            Some(ActiveRetainedDeleteResolution::from_request(&request));
        self.ui.sources.folders.delete_recovery.in_progress = true;
        self.show_status_progress(
            crate::app::state::ProgressTaskKind::FileOps,
            mode.progress_title(),
            request.entries.len(),
            false,
        );
        self.update_progress_detail(mode.progress_title());

        #[cfg(test)]
        {
            let result = run_retained_delete_resolution_job(request, None);
            self.apply_retained_delete_resolution_result(result);
            if self.ui.progress.task == Some(crate::app::state::ProgressTaskKind::FileOps) {
                self.clear_progress();
            }
        }

        #[cfg(not(test))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let cancel = Arc::new(AtomicBool::new(false));
            self.runtime.jobs.start_file_ops(rx, cancel);
            std::thread::spawn(move || {
                let result = run_retained_delete_resolution_job(request, Some(&tx));
                let _ = tx.send(FileOpMessage::Finished(
                    FileOpResult::RetainedDeleteResolution(result),
                ));
            });
        }
        Ok(true)
    }

    fn retained_delete_resolution_request(
        &self,
        mode: RetainedDeleteResolutionMode,
    ) -> RetainedDeleteResolutionRequest {
        RetainedDeleteResolutionRequest {
            mode,
            sources: self
                .library
                .sources
                .iter()
                .map(|source| RetainedDeleteResolutionSource {
                    source_id: source.id.clone(),
                    source_root: source.root.clone(),
                })
                .collect(),
            entries: self
                .ui
                .sources
                .folders
                .delete_recovery
                .retained_entries
                .iter()
                .map(retained_delete_resolution_entry)
                .collect(),
        }
    }

    pub(crate) fn apply_retained_delete_resolution_result(
        &mut self,
        result: RetainedDeleteResolutionResult,
    ) {
        self.runtime.active_retained_delete_resolution = None;
        let affected_sources: HashSet<SourceId> = result.affected_sources.iter().cloned().collect();
        let recovery_errors = self.apply_folder_delete_recovery_state(result.recovery_report);
        for source_id in &result.scan_sources {
            self.request_hard_sync_for_source(source_id);
        }
        self.refresh_recovered_sources(&affected_sources);
        for error in &result.failures {
            warn!(error = %error, "Retained folder delete resolution error");
        }
        for error in &recovery_errors {
            warn!(error = %error, "Delete recovery error");
        }
        let error_count = result.failures.len() + recovery_errors.len();
        self.set_status(
            status_message(result.mode, result.resolved, error_count),
            if error_count == 0 {
                StatusTone::Info
            } else {
                StatusTone::Warning
            },
        );
    }

    pub(crate) fn warn_if_retained_delete_path_busy(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        action: &str,
    ) -> bool {
        let Some(entry) = self
            .retained_delete_busy_entry(source_id, relative_path)
            .cloned()
        else {
            return false;
        };
        self.set_status(
            format!(
                "Recovery is still {} {} in {}; wait before {} {}",
                entry.mode.busy_verb(),
                entry.relative_path.display(),
                entry.source_label,
                action,
                relative_path.display()
            ),
            StatusTone::Warning,
        );
        true
    }

    fn retained_delete_busy_entry(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> Option<&RetainedDeleteBusyEntry> {
        self.runtime
            .active_retained_delete_resolution
            .as_ref()?
            .entries
            .iter()
            .find(|entry| &entry.source_id == source_id && entry.contains_path(relative_path))
    }

    #[cfg(test)]
    fn restore_retained_folder_delete(
        &mut self,
        entry: &UiRetainedFolderDeleteEntry,
        scan_sources: &mut HashSet<SourceId>,
    ) -> Result<SourceId, String> {
        let request = RetainedDeleteResolutionRequest {
            mode: RetainedDeleteResolutionMode::Restore,
            sources: vec![RetainedDeleteResolutionSource {
                source_id: entry.source_id.clone(),
                source_root: entry.source_root.clone(),
            }],
            entries: vec![retained_delete_resolution_entry(entry)],
        };
        let result = run_retained_delete_resolution_job(request, None);
        if let Some(err) = result.failures.into_iter().next() {
            return Err(err);
        }
        if result
            .scan_sources
            .iter()
            .any(|source_id| source_id == &entry.source_id)
        {
            scan_sources.insert(entry.source_id.clone());
        }
        Ok(entry.source_id.clone())
    }
}

fn retained_delete_resolution_entry(
    entry: &UiRetainedFolderDeleteEntry,
) -> RetainedDeleteResolutionEntry {
    RetainedDeleteResolutionEntry {
        id: entry.id.clone(),
        source_id: entry.source_id.clone(),
        source_root: entry.source_root.clone(),
        source_label: entry.source_label.clone(),
        relative_path: entry.relative_path.clone(),
        staged_relative: entry.staged_relative.clone(),
        deleted_entries: entry.deleted_entries.clone(),
    }
}

fn status_message(
    resolution: RetainedDeleteResolutionMode,
    resolved: usize,
    failures: usize,
) -> String {
    let label = resolution.status_label();
    if failures == 0 {
        return format!("{label} {resolved} retained folder delete(s)");
    }
    format!("{label} {resolved} retained folder delete(s) ({failures} error(s))")
}

#[cfg(test)]
mod tests;
