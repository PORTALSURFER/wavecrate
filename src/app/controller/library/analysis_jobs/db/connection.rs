use rusqlite::Connection;
use std::path::Path;

pub(crate) fn open_source_db(source_root: &Path) -> Result<Connection, String> {
    crate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|err| format!("Open source DB failed: {err}"))
}

/// Open a read-only source DB connection for long-lived or latency-sensitive UI queries.
pub(crate) fn open_source_db_ui_read(source_root: &Path) -> Result<Connection, String> {
    crate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::UiRead,
    )
    .map_err(|err| format!("Open source DB failed: {err}"))
}

/// Open a full maintenance connection for cleanup and deferred schema-sensitive work.
pub(crate) fn open_source_db_maintenance(source_root: &Path) -> Result<Connection, String> {
    crate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::Maintenance,
    )
    .map_err(|err| format!("Open source DB failed: {err}"))
}
