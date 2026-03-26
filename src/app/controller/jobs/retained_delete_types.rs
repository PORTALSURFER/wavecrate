//! DTOs for explicit retained-delete restore/purge background work.

use crate::app::controller::library::source_folders::delete_recovery::DeleteRecoveryReport;
use crate::sample_sources::{SourceId, WavEntry};
use std::path::{Path, PathBuf};

/// Requested retained-delete resolution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetainedDeleteResolutionMode {
    /// Restore retained folders back into the source tree.
    Restore,
    /// Permanently purge retained folders from staging.
    Purge,
}

impl RetainedDeleteResolutionMode {
    /// Past-tense label for summary/status text.
    pub(crate) const fn status_label(self) -> &'static str {
        match self {
            Self::Restore => "Restored",
            Self::Purge => "Purged",
        }
    }

    /// Continuous verb used while the job is active.
    pub(crate) const fn busy_verb(self) -> &'static str {
        match self {
            Self::Restore => "restoring",
            Self::Purge => "purging",
        }
    }

    /// Title shown in the shared file-op progress UI.
    pub(crate) const fn progress_title(self) -> &'static str {
        match self {
            Self::Restore => "Restoring retained deletes",
            Self::Purge => "Purging retained deletes",
        }
    }
}

/// Source root tracked by a retained-delete resolution request.
#[derive(Debug, Clone)]
pub(crate) struct RetainedDeleteResolutionSource {
    /// Stable source identifier.
    pub(crate) source_id: SourceId,
    /// Absolute source root path.
    pub(crate) source_root: PathBuf,
}

/// One retained folder queued for explicit restore or purge.
#[derive(Debug, Clone)]
pub(crate) struct RetainedDeleteResolutionEntry {
    /// Stable journal entry id.
    pub(crate) id: String,
    /// Owning source identifier.
    pub(crate) source_id: SourceId,
    /// Owning source root path.
    pub(crate) source_root: PathBuf,
    /// Display label for progress and warnings.
    pub(crate) source_label: String,
    /// Original retained folder path relative to the source root.
    pub(crate) relative_path: PathBuf,
    /// Staged folder path relative to `.sempal_delete_staging`.
    pub(crate) staged_relative: PathBuf,
    /// Deleted wav metadata snapshot used to rebuild DB state on restore.
    pub(crate) deleted_entries: Vec<WavEntry>,
}

/// Background request for explicit retained-delete resolution.
#[derive(Debug, Clone)]
pub(crate) struct RetainedDeleteResolutionRequest {
    /// Requested resolution mode.
    pub(crate) mode: RetainedDeleteResolutionMode,
    /// Loaded sources used to refresh retained-delete state after completion.
    pub(crate) sources: Vec<RetainedDeleteResolutionSource>,
    /// Retained entries to resolve.
    pub(crate) entries: Vec<RetainedDeleteResolutionEntry>,
}

/// Folder subtree that should warn while retained-delete resolution is active.
#[derive(Debug, Clone)]
pub(crate) struct RetainedDeleteBusyEntry {
    /// Resolution mode currently touching the folder.
    pub(crate) mode: RetainedDeleteResolutionMode,
    /// Source identifier that owns the busy folder.
    pub(crate) source_id: SourceId,
    /// Display label for warning text.
    pub(crate) source_label: String,
    /// Busy folder path relative to the source root.
    pub(crate) relative_path: PathBuf,
}

impl RetainedDeleteBusyEntry {
    /// Return true when the provided path lies inside this busy folder subtree.
    pub(crate) fn contains_path(&self, relative_path: &Path) -> bool {
        relative_path.starts_with(&self.relative_path)
    }
}

/// Runtime snapshot of the explicit retained-delete job currently in flight.
#[derive(Debug, Clone)]
pub(crate) struct ActiveRetainedDeleteResolution {
    /// Busy folder subtrees covered by the operation.
    pub(crate) entries: Vec<RetainedDeleteBusyEntry>,
}

impl ActiveRetainedDeleteResolution {
    /// Build the runtime busy-scope snapshot for one queued request.
    pub(crate) fn from_request(request: &RetainedDeleteResolutionRequest) -> Self {
        Self {
            entries: request
                .entries
                .iter()
                .map(|entry| RetainedDeleteBusyEntry {
                    mode: request.mode,
                    source_id: entry.source_id.clone(),
                    source_label: entry.source_label.clone(),
                    relative_path: entry.relative_path.clone(),
                })
                .collect(),
        }
    }
}

/// Background result for explicit retained-delete resolution.
#[derive(Debug)]
pub(crate) struct RetainedDeleteResolutionResult {
    /// Requested resolution mode that produced this result.
    pub(crate) mode: RetainedDeleteResolutionMode,
    /// Number of retained entries that finished successfully.
    pub(crate) resolved: usize,
    /// Sources whose visible state should be refreshed when the job completes.
    pub(crate) affected_sources: Vec<SourceId>,
    /// Sources that need a follow-up hard sync because metadata had to be inferred.
    pub(crate) scan_sources: Vec<SourceId>,
    /// Per-entry errors captured during resolution.
    pub(crate) failures: Vec<String>,
    /// Refreshed retained-delete report captured after the worker finishes.
    pub(crate) recovery_report: DeleteRecoveryReport,
}
