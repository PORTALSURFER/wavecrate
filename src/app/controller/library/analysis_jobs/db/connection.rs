use rusqlite::Connection;
use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

/// Writable source-database session for analysis workers and enqueue operations.
pub(crate) struct AnalysisJobSession(Connection);

/// Read-only source-database session for UI progress and side-effect-free cached queries.
pub(crate) struct AnalysisReadSession(Connection);

/// Writable source-database session for deferred cleanup and schema-sensitive work.
pub(crate) struct AnalysisMaintenanceSession(Connection);

macro_rules! impl_session_deref {
    ($session:ty) => {
        impl Deref for $session {
            type Target = Connection;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

impl_session_deref!(AnalysisJobSession);
impl_session_deref!(AnalysisReadSession);
impl_session_deref!(AnalysisMaintenanceSession);

impl DerefMut for AnalysisJobSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for AnalysisReadSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for AnalysisMaintenanceSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) fn open_source_db(source_root: &Path) -> Result<AnalysisJobSession, String> {
    crate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    )
    .map(AnalysisJobSession)
    .map_err(|err| format!("Open source DB failed: {err}"))
}

/// Open a read-only source DB connection for long-lived or latency-sensitive UI queries.
pub(crate) fn open_source_db_ui_read(source_root: &Path) -> Result<AnalysisReadSession, String> {
    crate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::UiRead,
    )
    .map(AnalysisReadSession)
    .map_err(|err| format!("Open source DB failed: {err}"))
}

/// Open a full maintenance connection for cleanup and deferred schema-sensitive work.
pub(crate) fn open_source_db_maintenance(
    source_root: &Path,
) -> Result<AnalysisMaintenanceSession, String> {
    crate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::Maintenance,
    )
    .map(AnalysisMaintenanceSession)
    .map_err(|err| format!("Open source DB failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_sessions_enforce_read_only_and_writable_profiles() {
        let root = tempfile::tempdir().expect("source root");
        let worker = open_source_db(root.path()).expect("worker session");
        worker
            .execute_batch("CREATE TABLE IF NOT EXISTS role_probe (value INTEGER);")
            .expect("worker session should write");
        drop(worker);

        let reader = open_source_db_ui_read(root.path()).expect("read session");
        let write_error = reader.execute("INSERT INTO role_probe (value) VALUES (1)", []);
        assert!(write_error.is_err(), "UI-read sessions must reject writes");

        let maintenance = open_source_db_maintenance(root.path()).expect("maintenance session");
        maintenance
            .execute("INSERT INTO role_probe (value) VALUES (2)", [])
            .expect("maintenance session should write");
    }
}
