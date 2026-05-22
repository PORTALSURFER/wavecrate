use std::path::Path;

use rusqlite::Connection;

use super::paths;
use crate::sample_sources::db::{
    SourceDatabase, SourceDatabaseConnectionRole, SourceDbError, telemetry,
};

pub(super) fn open_read_only_source_database(
    root: &Path,
    role: SourceDatabaseConnectionRole,
) -> Result<SourceDatabase, SourceDbError> {
    let open_started = std::time::Instant::now();
    if !root.is_dir() {
        return Err(SourceDbError::InvalidRoot(root.to_path_buf()));
    }

    let db_path = paths::read_only_db_path(root);
    if !db_path.is_file() {
        return Err(SourceDbError::ReadOnlyDatabaseMissing(db_path));
    }

    let connect_started = std::time::Instant::now();
    let connection = match Connection::open_with_flags(&db_path, role.open_flags()) {
        Ok(connection) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                role.label(),
                "connect",
                true,
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
                role.label(),
                "connect",
                true,
                connect_started.elapsed(),
                Err(&err),
            );
            telemetry::record_open_total(
                root,
                &db_path,
                role.label(),
                true,
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
        telemetry_label: role.label(),
    };
    let pragmas_started = std::time::Instant::now();
    if let Err(err) = db.apply_read_only_pragmas() {
        telemetry::record_open_phase(
            root,
            &db_path,
            role.label(),
            "pragmas",
            true,
            pragmas_started.elapsed(),
            Err(&err),
        );
        telemetry::record_open_total(
            root,
            &db_path,
            role.label(),
            true,
            open_started.elapsed(),
            Err(&err),
        );
        return Err(err);
    }
    telemetry::record_open_phase(
        root,
        &db_path,
        role.label(),
        "pragmas",
        true,
        pragmas_started.elapsed(),
        Ok(()),
    );
    telemetry::record_open_total(
        root,
        &db_path,
        role.label(),
        true,
        open_started.elapsed(),
        Ok(()),
    );
    Ok(db)
}
