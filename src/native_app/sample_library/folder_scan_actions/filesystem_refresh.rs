use std::{path::PathBuf, time::Instant};

use radiant::prelude as ui;

use crate::native_app::app::{
    GuiMessage, NativeAppState, SourceFilesystemChangePlan, SourceFilesystemSyncResult,
    SourceRefreshRequest, emit_gui_action,
};
use crate::native_app::sample_library::folder_scan_actions::filesystem_refresh_worker::sync_source_database_paths;
use crate::native_app::sample_library::source_prep::SourcePrepTrigger;

impl NativeAppState {
    pub(in crate::native_app) fn refresh_source_after_filesystem_change(
        &mut self,
        source_id: String,
        paths: Vec<PathBuf>,
        overflowed: bool,
        source_root_available: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self.library.plan_filesystem_change(
            source_id,
            &paths,
            overflowed,
            source_root_available,
        ) {
            SourceFilesystemChangePlan::IgnoredSourceMissing { source_id } => {
                self.background
                    .source_processing
                    .wake_source(&source_id, "source_root_availability_changed");
                if source_id == self.library.folder_browser.selected_source_id() {
                    self.ui.status.sample = String::from("Source missing");
                }
                self.persist_user_configuration(
                    "folder_browser.source.availability_changed",
                    started_at,
                );
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "ignored",
                    started_at,
                    Some("source_not_found"),
                );
            }
            SourceFilesystemChangePlan::SyncPaths {
                source_id,
                changed_count,
            } => {
                self.queue_source_filesystem_sync(source_id.clone(), paths, changed_count, context);
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "sync_queued",
                    started_at,
                    None,
                );
            }
            SourceFilesystemChangePlan::DeferredAlreadyRunning { source_id } => {
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "deferred",
                    started_at,
                    Some("scan_already_running"),
                );
            }
            SourceFilesystemChangePlan::QueueRefresh { source_id } => {
                self.queue_filesystem_source_refresh(source_id, started_at, context);
            }
        }
    }

    pub(in crate::native_app) fn finish_source_filesystem_sync(
        &mut self,
        result: SourceFilesystemSyncResult,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let source_id = result.source_id;
        if self.background.source_lifecycle_generations.get(&source_id)
            != Some(&result.lifecycle_generation)
        {
            tracing::debug!(
                source_id = %source_id,
                lifecycle_generation = result.lifecycle_generation,
                "Ignoring filesystem sync completion from an inactive source generation"
            );
            return;
        }
        let changed_count = result.changed_count;
        if !self.library.folder_browser.source_exists(&source_id) {
            tracing::debug!(
                source_id = %source_id,
                "Ignoring stale filesystem sync completion for removed source"
            );
            return;
        }
        match result.result {
            Ok(success) => {
                let renames_reconciled = success.renames_reconciled;
                let incomplete_error = success.incomplete_error;
                let delta = success.committed_delta;
                tracing::info!(
                    source_id = %source_id,
                    revision = delta.revision,
                    created = delta.created.len(),
                    changed = delta.changed.len(),
                    moved = delta.moved.len(),
                    deleted = delta.deleted.len(),
                    renames_reconciled,
                    "Committed filesystem source delta"
                );
                if !delta.is_empty() && incomplete_error.is_none() {
                    self.ui.status.sample = format!("Synced {changed_count} filesystem change(s)");
                    self.queue_source_prep(
                        source_id.clone(),
                        SourcePrepTrigger::FilesystemChanged,
                        context,
                    );
                }
                if result.cancelled || incomplete_error.is_some() {
                    self.background
                        .source_processing
                        .wake_source(&source_id, "filesystem_sync_incomplete_after_commit");
                }
                self.queue_filesystem_source_refresh(source_id, Instant::now(), context);
            }
            Err(error) => {
                tracing::warn!(
                    source_id = %source_id,
                    changed_count,
                    error = %error,
                    "Failed to sync source database after filesystem change"
                );
                if source_id == self.library.folder_browser.selected_source_id() {
                    self.ui.status.sample = format!("Source sync failed: {error}");
                }
                self.queue_filesystem_source_refresh(source_id, Instant::now(), context);
            }
        }
    }

    pub(in crate::native_app) fn finish_source_manifest_audit(
        &mut self,
        source_id: String,
        lifecycle_generation: u64,
        committed_delta: wavecrate::sample_sources::scanner::CommittedSourceDelta,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.background.source_lifecycle_generations.get(&source_id)
            != Some(&lifecycle_generation)
            || committed_delta.is_empty()
            || !self.library.folder_browser.source_exists(&source_id)
        {
            return;
        }
        self.background
            .source_processing
            .wake_source(&source_id, "manifest_audit_committed");
        if !manifest_delta_requires_browser_refresh(&committed_delta) {
            tracing::debug!(
                source_id = %source_id,
                revision = committed_delta.revision,
                "Skipping filesystem rescan for content-generation-only audit delta"
            );
            return;
        }
        tracing::info!(
            source_id = %source_id,
            revision = committed_delta.revision,
            created = committed_delta.created.len(),
            changed = committed_delta.changed.len(),
            moved = committed_delta.moved.len(),
            deleted = committed_delta.deleted.len(),
            "Refreshing browser projection after periodic source audit"
        );
        self.queue_filesystem_source_refresh(source_id, Instant::now(), context);
    }

    pub(in crate::native_app) fn maybe_run_pending_source_refresh(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if let Some(source_id) = self.library.next_pending_source_refresh_if_idle() {
            self.queue_filesystem_source_refresh(source_id, Instant::now(), context);
        }
    }

    fn queue_filesystem_source_refresh(
        &mut self,
        source_id: String,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let task_id = self.next_folder_task_id();
        match self
            .library
            .begin_filesystem_refresh(source_id.clone(), task_id)
        {
            SourceRefreshRequest::Queued(request) => {
                let label = request.label.clone();
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&label),
                    "scan_queued",
                    started_at,
                    None,
                );
                self.launch_folder_scan(request, context);
            }
            SourceRefreshRequest::Deferred { source_id } => {
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "deferred",
                    started_at,
                    Some("source_not_queued"),
                );
            }
            SourceRefreshRequest::IgnoredMissing { source_id } => {
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "ignored_missing",
                    started_at,
                    Some("source_root_missing"),
                );
            }
        }
    }

    fn queue_source_filesystem_sync(
        &mut self,
        source_id: String,
        paths: Vec<PathBuf>,
        changed_count: usize,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if paths.is_empty() {
            return;
        }
        let (root, database_root, expected_lifecycle_generation) =
            match self.admit_source_filesystem_sync(&source_id) {
                Ok(admission) => admission,
                Err(error) => {
                    tracing::warn!(
                        target: "wavecrate::source_processing",
                        source_id,
                        error,
                        "Source filesystem sync was not admitted"
                    );
                    return;
                }
            };
        let budget = self.background.source_processing.budget_handle();
        context.business().background("gui-source-db-sync").run(
            move |_| {
                let Some(permit) =
                    budget.acquire_scan_for_generation(&source_id, expected_lifecycle_generation)
                else {
                    return SourceFilesystemSyncResult {
                        source_id,
                        lifecycle_generation: expected_lifecycle_generation,
                        changed_count,
                        cancelled: true,
                        result: Err(String::from("Source filesystem sync canceled")),
                    };
                };
                let lifecycle_generation = permit.lifecycle_generation();
                let cancel = permit.cancel_token();
                let mut result = sync_source_database_paths(
                    source_id,
                    root,
                    database_root,
                    paths,
                    changed_count,
                    cancel.as_ref(),
                );
                result.lifecycle_generation = lifecycle_generation;
                drop(permit);
                result
            },
            GuiMessage::SourceFilesystemSyncFinished,
        );
    }

    pub(in crate::native_app) fn admit_source_filesystem_sync(
        &mut self,
        source_id: &str,
    ) -> Result<(PathBuf, PathBuf, u64), String> {
        let source = self
            .library
            .folder_browser
            .configured_sample_sources()
            .into_iter()
            .find(|source| source.id.as_str() == source_id)
            .ok_or_else(|| "Source is not present in the configured source set".to_string())?;
        let root = source.root.clone();
        let database_root = source
            .database_root()
            .map_err(|error| format!("Resolve source metadata location failed: {error}"))?;
        let lifecycle_generation = self
            .background
            .source_processing
            .register_source_for_scan(source)?;
        self.background
            .source_lifecycle_generations
            .insert(source_id.to_string(), lifecycle_generation);
        Ok((root, database_root, lifecycle_generation))
    }
}

fn manifest_delta_requires_browser_refresh(
    delta: &wavecrate::sample_sources::scanner::CommittedSourceDelta,
) -> bool {
    !delta.created.is_empty()
        || !delta.moved.is_empty()
        || !delta.deleted.is_empty()
        || delta
            .changed
            .iter()
            .any(|change| change.source_metadata_changed)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::manifest_delta_requires_browser_refresh;
    use wavecrate::sample_sources::scanner::{CommittedSourceDelta, ManifestIdentityDelta};

    #[test]
    fn content_generation_only_audit_does_not_queue_filesystem_rescan() {
        let delta = CommittedSourceDelta {
            revision: 7,
            changed: vec![ManifestIdentityDelta {
                identity: String::from("file-id"),
                relative_path: PathBuf::from("sample.wav"),
                content_generation: String::from("hash"),
                source_metadata_changed: false,
            }],
            ..CommittedSourceDelta::default()
        };

        assert!(!manifest_delta_requires_browser_refresh(&delta));
    }

    #[test]
    fn source_metadata_change_requires_browser_refresh() {
        let delta = CommittedSourceDelta {
            revision: 8,
            changed: vec![ManifestIdentityDelta {
                identity: String::from("file-id"),
                relative_path: PathBuf::from("sample.wav"),
                content_generation: String::from("new-hash"),
                source_metadata_changed: true,
            }],
            ..CommittedSourceDelta::default()
        };

        assert!(manifest_delta_requires_browser_refresh(&delta));
    }
}
