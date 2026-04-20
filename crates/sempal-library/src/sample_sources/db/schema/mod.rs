//! SQLite schema creation and migration helpers for source databases.

use rusqlite::Connection;

use super::SourceDbError;
use super::util::map_sql_error;

mod ddl;
mod migrations;

/// SQLite `user_version` value for the current source-db schema shape.
///
/// A matching stamp means the base DDL, additive migrations, and indices have
/// already been assured for this file, so steady-state opens can skip that
/// one-time work.
const SOURCE_DB_SCHEMA_VERSION: i64 = 1;

/// Apply the full source-database schema, including deferred cleanup work.
pub(super) fn apply_schema(connection: &Connection) -> Result<SchemaApplyOutcome, SourceDbError> {
    apply_schema_with_mode(connection, SchemaApplyMode::Full)
}

/// Apply the source-database schema using only startup-friendly migration work.
pub(super) fn apply_schema_fast(
    connection: &Connection,
) -> Result<SchemaApplyOutcome, SourceDbError> {
    apply_schema_with_mode(connection, SchemaApplyMode::Fast)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SchemaApplyMode {
    Fast,
    Full,
}

/// Outcome describing whether schema assurance ran for this open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SchemaApplyOutcome {
    /// The connection performed schema assurance because the database was new or stale.
    Assured,
    /// The database already carried the current schema stamp, so assurance was skipped.
    Current,
}

fn apply_schema_with_mode(
    connection: &Connection,
    mode: SchemaApplyMode,
) -> Result<SchemaApplyOutcome, SourceDbError> {
    let outcome = if schema_is_current(connection)? {
        SchemaApplyOutcome::Current
    } else {
        assure_schema(connection)?;
        SchemaApplyOutcome::Assured
    };
    if mode == SchemaApplyMode::Full {
        migrations::remove_invalid_relative_paths(connection)?;
    }
    Ok(outcome)
}

fn assure_schema(connection: &Connection) -> Result<(), SourceDbError> {
    ddl::apply_base_schema(connection)?;
    migrations::apply_optional_migrations(connection)?;
    ddl::apply_indices(connection)?;
    stamp_schema_version(connection)?;
    Ok(())
}

fn schema_is_current(connection: &Connection) -> Result<bool, SourceDbError> {
    Ok(read_schema_version(connection)? == SOURCE_DB_SCHEMA_VERSION)
}

fn read_schema_version(connection: &Connection) -> Result<i64, SourceDbError> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_sql_error)
}

fn stamp_schema_version(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .pragma_update(None, "user_version", SOURCE_DB_SCHEMA_VERSION)
        .map_err(map_sql_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OptionalExtension;

    fn index_exists(connection: &Connection, index_name: &str) -> bool {
        connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [index_name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .unwrap()
            .is_some()
    }

    #[test]
    fn stamped_databases_skip_repeat_schema_assurance() {
        let connection = Connection::open_in_memory().unwrap();

        let first = apply_schema_fast(&connection).unwrap();
        assert_eq!(first, SchemaApplyOutcome::Assured);
        assert_eq!(
            read_schema_version(&connection).unwrap(),
            SOURCE_DB_SCHEMA_VERSION
        );
        connection
            .execute("DROP INDEX idx_wav_files_missing", [])
            .unwrap();
        assert!(!index_exists(&connection, "idx_wav_files_missing"));

        let second = apply_schema_fast(&connection).unwrap();
        assert_eq!(second, SchemaApplyOutcome::Current);
        assert!(!index_exists(&connection, "idx_wav_files_missing"));
    }

    #[test]
    fn stale_schema_stamp_reapplies_schema_assurance() {
        let connection = Connection::open_in_memory().unwrap();
        apply_schema_fast(&connection).unwrap();
        connection
            .execute("DROP INDEX idx_wav_files_missing", [])
            .unwrap();
        connection.pragma_update(None, "user_version", 0).unwrap();

        let outcome = apply_schema_fast(&connection).unwrap();
        assert_eq!(outcome, SchemaApplyOutcome::Assured);
        assert!(index_exists(&connection, "idx_wav_files_missing"));
        assert_eq!(
            read_schema_version(&connection).unwrap(),
            SOURCE_DB_SCHEMA_VERSION
        );
    }
}
