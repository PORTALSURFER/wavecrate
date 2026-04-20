use rusqlite::OpenFlags;

use super::SourceDatabaseOpenMode;

/// Explicit source-database runtime roles used to scope connection behavior.
///
/// Each role intentionally owns a very small policy surface so high-level app
/// code can declare whether it is opening the source DB for a long-lived UI
/// reader, a short-lived analysis worker, or a deferred maintenance sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceDatabaseConnectionRole {
    /// Read-only connection profile for cached UI queries and progress polling.
    UiRead,
    /// Read-write profile for analysis enqueue/claim/finalization workers.
    JobWorker,
    /// Read-write profile for deferred cleanup and schema-sensitive maintenance.
    Maintenance,
}

impl SourceDatabaseConnectionRole {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::UiRead => "ui_read",
            Self::JobWorker => "job_worker",
            Self::Maintenance => "maintenance",
        }
    }

    pub(super) fn open_flags(self) -> OpenFlags {
        match self {
            Self::UiRead => OpenFlags::SQLITE_OPEN_READ_ONLY,
            Self::JobWorker | Self::Maintenance => {
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
            }
        }
    }

    pub(super) fn open_mode(self) -> SourceDatabaseOpenMode {
        match self {
            Self::UiRead | Self::JobWorker => SourceDatabaseOpenMode::Fast,
            Self::Maintenance => SourceDatabaseOpenMode::Full,
        }
    }

    pub(super) fn uses_read_only_connection(self) -> bool {
        matches!(self, Self::UiRead)
    }
}
