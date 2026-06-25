//! Incremental schema migrations for older source database files.

use std::collections::HashSet;

use rusqlite::Connection;

use super::super::SourceDbError;
use super::super::util::map_sql_error;

mod analysis_jobs;
mod aspect_descriptors;
mod columns;
mod invalid_paths;
mod tag_catalog;

use self::analysis_jobs::{
    ensure_analysis_job_progress_snapshots, ensure_analysis_jobs_optional_columns,
};
use self::aspect_descriptors::ensure_aspect_descriptor_tables;
use self::columns::{
    ensure_feature_metric_columns, ensure_file_ops_journal_last_curated_at_column,
    ensure_file_ops_journal_optional_columns, ensure_pending_rename_optional_columns,
    ensure_samples_optional_columns, ensure_wav_files_collection_column,
    ensure_wav_files_last_curated_at_column, ensure_wav_files_optional_columns,
};
pub(super) use self::invalid_paths::remove_invalid_relative_paths;
use self::tag_catalog::{backfill_tag_catalog, ensure_tag_catalog_schema};

/// Apply additive column migrations needed by older source databases.
pub(super) fn apply_optional_migrations(connection: &Connection) -> Result<(), SourceDbError> {
    ensure_wav_files_optional_columns(connection)?;
    ensure_pending_rename_optional_columns(connection)?;
    ensure_file_ops_journal_optional_columns(connection)?;
    ensure_analysis_jobs_optional_columns(connection)?;
    ensure_analysis_job_progress_snapshots(connection)?;
    ensure_samples_optional_columns(connection)?;
    ensure_feature_metric_columns(connection, "features")?;
    ensure_feature_metric_columns(connection, "analysis_cache_features")?;
    ensure_tag_catalog_schema(connection)?;
    backfill_tag_catalog(connection)?;
    ensure_collection_membership_schema(connection)?;
    ensure_aspect_descriptor_tables(connection)?;
    Ok(())
}

/// Apply low-cost additive repairs that must not be skipped for current stamps.
pub(super) fn apply_current_stamp_repairs(connection: &Connection) -> Result<(), SourceDbError> {
    ensure_wav_files_collection_column(connection)?;
    ensure_wav_files_last_curated_at_column(connection)?;
    ensure_file_ops_journal_last_curated_at_column(connection)?;
    ensure_collection_membership_schema(connection)?;
    ensure_pending_rename_optional_columns(connection)?;
    ensure_aspect_descriptor_tables(connection)?;
    Ok(())
}

fn ensure_collection_membership_schema(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS wav_file_collections (
                path TEXT NOT NULL,
                collection INTEGER NOT NULL,
                PRIMARY KEY (path, collection),
                FOREIGN KEY(path) REFERENCES wav_files(path) ON DELETE CASCADE
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_wav_file_collections_collection
                ON wav_file_collections(collection);
            INSERT OR IGNORE INTO wav_file_collections (path, collection)
            SELECT path, collection
            FROM wav_files
            WHERE collection IS NOT NULL;",
        )
        .map_err(map_sql_error)?;
    Ok(())
}

pub(crate) fn table_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<HashSet<String>, SourceDbError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = connection.prepare(&pragma).map_err(map_sql_error)?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(map_sql_error)?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(map_sql_error)?;
    Ok(columns)
}

#[cfg(test)]
mod tests;
