use std::path::Path;

use rusqlite::{Connection, OpenFlags};

mod paths;
mod read_only;

use super::{
    SOURCE_DB_ALLOW_USER_LIBRARY_WRITE_ENV, SOURCE_DB_READ_ONLY_ENV, SourceDatabase,
    SourceDatabaseConnectionRole, SourceDbError, telemetry, util,
};
use read_only::open_read_only_source_database;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceDatabaseOpenMode {
    Fast,
    Full,
}

pub(super) fn open_source_database_for_role(
    root: &Path,
    allow_user_library_write: bool,
    role: SourceDatabaseConnectionRole,
) -> Result<SourceDatabase, SourceDbError> {
    if role.uses_read_only_connection() {
        return open_read_only_source_database(root, role);
    }
    open_source_database_with_flags(
        root,
        allow_user_library_write,
        role.open_flags(),
        role.open_mode(),
        role.label(),
    )
}

pub(crate) fn open_source_database(
    root: &Path,
    read_only: bool,
    allow_user_library_write: bool,
    mode: SourceDatabaseOpenMode,
) -> Result<SourceDatabase, SourceDbError> {
    if read_only {
        return open_read_only_source_database(root, SourceDatabaseConnectionRole::UiRead);
    }
    open_source_database_with_flags(
        root,
        allow_user_library_write,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        mode,
        mode.label(),
    )
}

fn open_source_database_with_flags(
    root: &Path,
    allow_user_library_write: bool,
    open_flags: OpenFlags,
    mode: SourceDatabaseOpenMode,
    telemetry_label: &'static str,
) -> Result<SourceDatabase, SourceDbError> {
    let open_started = std::time::Instant::now();
    if !root.is_dir() {
        return Err(SourceDbError::InvalidRoot(root.to_path_buf()));
    }

    if paths::is_user_library_root(root) && !allow_user_library_write {
        return Err(SourceDbError::UserLibraryWriteBlocked {
            path: root.to_path_buf(),
        });
    }

    let db_path = paths::prepare_writable_db_path(root)?;
    util::create_parent_if_needed(&db_path)?;
    let connect_started = std::time::Instant::now();
    let connection = match Connection::open_with_flags(&db_path, open_flags) {
        Ok(connection) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "connect",
                false,
                connect_started.elapsed(),
                Ok(()),
            );
            connection
        }
        Err(err) => {
            let err = SourceDbError::from(err);
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "connect",
                false,
                connect_started.elapsed(),
                Err(&err),
            );
            telemetry::record_open_total(
                root,
                &db_path,
                telemetry_label,
                false,
                open_started.elapsed(),
                Err(&err),
            );
            return Err(err);
        }
    };
    let db = SourceDatabase {
        connection,
        db_path: db_path.clone(),
        root: root.to_path_buf(),
        telemetry_label,
    };
    let pragmas_started = std::time::Instant::now();
    if let Err(err) = db.apply_pragmas() {
        telemetry::record_open_phase(
            root,
            &db_path,
            telemetry_label,
            "pragmas",
            false,
            pragmas_started.elapsed(),
            Err(&err),
        );
        telemetry::record_open_total(
            root,
            &db_path,
            telemetry_label,
            false,
            open_started.elapsed(),
            Err(&err),
        );
        return Err(err);
    }
    telemetry::record_open_phase(
        root,
        &db_path,
        telemetry_label,
        "pragmas",
        false,
        pragmas_started.elapsed(),
        Ok(()),
    );
    let schema_started = std::time::Instant::now();
    let schema_result = match mode {
        SourceDatabaseOpenMode::Fast => db.apply_schema_fast(),
        SourceDatabaseOpenMode::Full => db.apply_schema(),
    };
    match schema_result {
        Ok(()) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "schema",
                false,
                schema_started.elapsed(),
                Ok(()),
            );
        }
        Err(err) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "schema",
                false,
                schema_started.elapsed(),
                Err(&err),
            );
            telemetry::record_open_total(
                root,
                &db_path,
                telemetry_label,
                false,
                open_started.elapsed(),
                Err(&err),
            );
            return Err(err);
        }
    }
    telemetry::record_open_total(
        root,
        &db_path,
        telemetry_label,
        false,
        open_started.elapsed(),
        Ok(()),
    );
    Ok(db)
}

impl SourceDatabaseOpenMode {
    fn label(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Full => "full",
        }
    }
}

/// Reset the debug-only source DB open counter used by regression tests.
#[cfg(debug_assertions)]
pub fn test_reset_source_db_open_total_count(root: &Path) {
    telemetry::reset_open_total_count(root);
}

/// Return the debug-only source DB open count used by regression tests.
#[cfg(debug_assertions)]
pub fn test_source_db_open_total_count(root: &Path) -> usize {
    telemetry::open_total_count(root)
}

pub(super) fn should_open_source_db_read_only() -> bool {
    crate::env_flags::env_var_truthy(SOURCE_DB_READ_ONLY_ENV)
}

pub(super) fn allow_user_library_db_write() -> bool {
    crate::env_flags::env_var_truthy(SOURCE_DB_ALLOW_USER_LIBRARY_WRITE_ENV)
}
