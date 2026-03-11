//! SQLite schema creation and migration helpers for source databases.

use rusqlite::Connection;

use super::SourceDbError;

mod ddl;
mod migrations;

/// Apply the full source-database schema, including deferred cleanup work.
pub(super) fn apply_schema(connection: &Connection) -> Result<(), SourceDbError> {
    apply_schema_with_mode(connection, SchemaApplyMode::Full)
}

/// Apply the source-database schema using only startup-friendly migration work.
pub(super) fn apply_schema_fast(connection: &Connection) -> Result<(), SourceDbError> {
    apply_schema_with_mode(connection, SchemaApplyMode::Fast)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SchemaApplyMode {
    Fast,
    Full,
}

fn apply_schema_with_mode(
    connection: &Connection,
    mode: SchemaApplyMode,
) -> Result<(), SourceDbError> {
    ddl::apply_base_schema(connection)?;
    migrations::apply_optional_migrations(connection)?;
    if mode == SchemaApplyMode::Full {
        migrations::remove_invalid_relative_paths(connection)?;
    }
    ddl::apply_indices(connection)?;
    Ok(())
}
