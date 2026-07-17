//! Authoritative completion contract for Wavecrate-owned filesystem mutations.
//!
//! File-operation workers own the filesystem and operation-specific rollback. Once a worker has
//! reached its durable filesystem boundary, this module reconciles every affected source database,
//! publishes one revisioned outcome, refreshes the browser projection from that committed state,
//! acknowledges the matching watcher echo, and only then wakes durable readiness reconciliation.

use std::{collections::BTreeSet, path::PathBuf};

use radiant::prelude as ui;
use wavecrate::sample_sources::{readiness::ReadinessStage, scanner::CommittedSourceDelta};

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::sample_library::source_prep::SourcePrepTrigger;

#[cfg(test)]
mod tests;
mod watcher_echo;
mod worker;

pub(in crate::native_app) use watcher_echo::{
    CommittedWatcherEcho, CommittedWatcherPathState, observed_watcher_path_state,
};
use worker::{
    build_source_requests, merge_file_mutation_failures, mutation_completion_is_stale_or_duplicate,
    reconcile_file_mutation_requests,
};

/// User-visible mutation family that owns one operation ID across all affected sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FileMutationOperation {
    Duplicate,
    Extract,
    ImportDrop,
    Edit,
    Normalize,
    Undo,
    Redo,
    Rename,
    Move,
    Trash,
}

impl FileMutationOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate",
            Self::Extract => "extract",
            Self::ImportDrop => "import_drop",
            Self::Edit => "edit",
            Self::Normalize => "normalize",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::Rename => "rename",
            Self::Move => "move",
            Self::Trash => "trash",
        }
    }
}

/// Readiness-relevant meaning of one committed path transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FileMutationSemantics {
    Create,
    ContentChanged,
    PathOnlyMove,
    Delete,
}

/// One logical file or folder transition. Paths are absolute so cross-source moves retain both
/// endpoints in every source-scoped outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMutationChange {
    pub(in crate::native_app) before_path: Option<PathBuf>,
    pub(in crate::native_app) after_path: Option<PathBuf>,
    pub(in crate::native_app) before_content_identity: Option<String>,
    pub(in crate::native_app) after_content_identity: Option<String>,
    pub(in crate::native_app) semantics: FileMutationSemantics,
}

impl FileMutationChange {
    pub(in crate::native_app) fn created(path: PathBuf) -> Self {
        Self {
            before_path: None,
            after_path: Some(path),
            before_content_identity: None,
            after_content_identity: None,
            semantics: FileMutationSemantics::Create,
        }
    }

    pub(in crate::native_app) fn content_changed(path: PathBuf) -> Self {
        Self {
            before_path: Some(path.clone()),
            after_path: Some(path),
            before_content_identity: None,
            after_content_identity: None,
            semantics: FileMutationSemantics::ContentChanged,
        }
    }

    pub(in crate::native_app) fn path_only_move(before: PathBuf, after: PathBuf) -> Self {
        Self {
            before_path: Some(before),
            after_path: Some(after),
            before_content_identity: None,
            after_content_identity: None,
            semantics: FileMutationSemantics::PathOnlyMove,
        }
    }

    pub(in crate::native_app) fn deleted(path: PathBuf) -> Self {
        Self {
            before_path: Some(path),
            after_path: None,
            before_content_identity: None,
            after_content_identity: None,
            semantics: FileMutationSemantics::Delete,
        }
    }

    pub(in crate::native_app) fn with_before_content_identity(
        mut self,
        identity: Option<String>,
    ) -> Self {
        self.before_content_identity = identity;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct CommittedFileMutation {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) operation_id: u64,
    pub(in crate::native_app) operation: FileMutationOperation,
    pub(in crate::native_app) committed_source_revision: u64,
    pub(in crate::native_app) changes: Vec<FileMutationChange>,
    pub(in crate::native_app) invalidated_stages: BTreeSet<ReadinessStage>,
    pub(in crate::native_app) committed_delta: CommittedSourceDelta,
    pub(in crate::native_app) affected_relative_paths: Vec<PathBuf>,
    pub(in crate::native_app) watcher_echoes: Vec<CommittedWatcherEcho>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMutationFailure {
    pub(in crate::native_app) source_id: Option<String>,
    pub(in crate::native_app) operation_id: u64,
    pub(in crate::native_app) operation: FileMutationOperation,
    pub(in crate::native_app) error: String,
}

/// Explicit terminal outcome. A cross-source operation can commit one source and fail another;
/// readiness is woken only for entries in `Committed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FileMutationOutcome {
    Committed(Vec<CommittedFileMutation>),
    Failed {
        committed: Vec<CommittedFileMutation>,
        failures: Vec<FileMutationFailure>,
    },
    RolledBack(FileMutationFailure),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMutationWork {
    requests: Vec<worker::SourceMutationRequest>,
    failures: Vec<FileMutationFailure>,
}

impl NativeAppState {
    /// Reconcile one successful Wavecrate-owned filesystem operation off the UI thread.
    pub(in crate::native_app) fn queue_committed_file_mutation(
        &mut self,
        operation: FileMutationOperation,
        changes: Vec<FileMutationChange>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Option<u64> {
        self.queue_file_mutation_outcome(operation, changes, Vec::new(), context)
    }

    pub(in crate::native_app) fn queue_partially_committed_file_mutation(
        &mut self,
        operation: FileMutationOperation,
        changes: Vec<FileMutationChange>,
        failures: Vec<(Option<String>, String)>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Option<u64> {
        self.queue_file_mutation_outcome(operation, changes, failures, context)
    }

    fn queue_file_mutation_outcome(
        &mut self,
        operation: FileMutationOperation,
        changes: Vec<FileMutationChange>,
        reported_failures: Vec<(Option<String>, String)>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> Option<u64> {
        if changes.is_empty() && reported_failures.is_empty() {
            return None;
        }
        let operation_id = self.background.next_task_id();
        let had_changes = !changes.is_empty();
        let failures = reported_failures
            .into_iter()
            .map(|(source_id, error)| FileMutationFailure {
                source_id,
                operation_id,
                operation,
                error,
            })
            .collect::<Vec<_>>();
        let sources = self.library.folder_browser.configured_sample_sources();
        let requests = build_source_requests(operation_id, operation, changes, &sources);
        if requests.is_empty() {
            let mut failures = failures;
            if had_changes {
                failures.push(FileMutationFailure {
                    source_id: None,
                    operation_id,
                    operation,
                    error: String::from("No configured source owns the committed mutation paths"),
                });
            }
            self.finish_committed_file_mutation(
                FileMutationOutcome::Failed {
                    committed: Vec::new(),
                    failures,
                },
                context,
            );
            return Some(operation_id);
        }
        context.emit(GuiMessage::CommittedFileMutationRequested(
            FileMutationWork { requests, failures },
        ));
        Some(operation_id)
    }

    pub(in crate::native_app) fn start_committed_file_mutation(
        &mut self,
        work: FileMutationWork,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        context
            .business()
            .background("gui-committed-file-mutation")
            .run(
                move |_| {
                    merge_file_mutation_failures(
                        reconcile_file_mutation_requests(work.requests),
                        work.failures,
                    )
                },
                GuiMessage::CommittedFileMutationFinished,
            );
    }

    pub(in crate::native_app) fn record_failed_file_mutation(
        &mut self,
        operation: FileMutationOperation,
        source_id: Option<String>,
        error: impl Into<String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let operation_id = self.background.next_task_id();
        self.finish_committed_file_mutation(
            FileMutationOutcome::Failed {
                committed: Vec::new(),
                failures: vec![FileMutationFailure {
                    source_id,
                    operation_id,
                    operation,
                    error: error.into(),
                }],
            },
            context,
        );
    }

    pub(in crate::native_app) fn record_rolled_back_file_mutation(
        &mut self,
        operation: FileMutationOperation,
        source_id: Option<String>,
        error: impl Into<String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let operation_id = self.background.next_task_id();
        self.finish_committed_file_mutation(
            FileMutationOutcome::RolledBack(FileMutationFailure {
                source_id,
                operation_id,
                operation,
                error: error.into(),
            }),
            context,
        );
    }

    pub(in crate::native_app) fn finish_committed_file_mutation(
        &mut self,
        outcome: FileMutationOutcome,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let (committed, failures) = match outcome {
            FileMutationOutcome::Committed(committed) => (committed, Vec::new()),
            FileMutationOutcome::Failed {
                committed,
                failures,
            } => (committed, failures),
            FileMutationOutcome::RolledBack(failure) => {
                tracing::warn!(
                    operation_id = failure.operation_id,
                    operation = failure.operation.as_str(),
                    source_id = failure.source_id.as_deref().unwrap_or("unknown"),
                    error = %failure.error,
                    "Wavecrate-owned file mutation rolled back"
                );
                return;
            }
        };

        for event in committed {
            let last_commit = self
                .transactions
                .latest_committed_mutation
                .entry(event.source_id.clone())
                .or_default();
            let current_commit = (event.committed_source_revision, event.operation_id);
            if mutation_completion_is_stale_or_duplicate(*last_commit, current_commit) {
                tracing::debug!(
                    source_id = %event.source_id,
                    operation_id = event.operation_id,
                    revision = event.committed_source_revision,
                    accepted_revision = last_commit.0,
                    accepted_operation_id = last_commit.1,
                    "Ignoring stale committed file-mutation completion"
                );
                continue;
            }
            *last_commit = (*last_commit).max(current_commit);

            self.library
                .folder_browser
                .refresh_filesystem_paths(&event.source_id, &event.affected_relative_paths);
            if let Some(watcher) = self.library.source_watcher.as_ref() {
                watcher.acknowledge_committed_paths(
                    event.source_id.clone(),
                    event.watcher_echoes,
                    event.operation_id,
                );
            }
            tracing::info!(
                source_id = %event.source_id,
                operation_id = event.operation_id,
                operation = event.operation.as_str(),
                revision = event.committed_source_revision,
                changes = event.changes.len(),
                invalidated_stages = ?event.invalidated_stages,
                "Committed Wavecrate-owned file mutation"
            );
            // This call refreshes metadata projections and wakes the source-owned readiness
            // reconciler. It deliberately happens after the source DB and browser projection.
            self.queue_source_prep(
                event.source_id,
                SourcePrepTrigger::FilesystemChanged,
                context,
            );
        }

        for failure in failures {
            tracing::warn!(
                operation_id = failure.operation_id,
                operation = failure.operation.as_str(),
                source_id = failure.source_id.as_deref().unwrap_or("unknown"),
                error = %failure.error,
                "Wavecrate-owned file mutation failed before authoritative publication"
            );
        }
    }
}
