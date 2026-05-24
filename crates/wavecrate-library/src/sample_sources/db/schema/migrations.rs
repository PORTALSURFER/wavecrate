//! Incremental schema migrations for older source database files.

use std::collections::HashSet;

use rusqlite::Connection;

use super::super::SourceDbError;
use super::super::util::map_sql_error;

mod analysis_jobs;
mod columns;
mod invalid_paths;
mod tag_catalog;

use self::analysis_jobs::{
    ensure_analysis_job_progress_snapshots, ensure_analysis_jobs_optional_columns,
};
use self::columns::{
    ensure_feature_metric_columns, ensure_file_ops_journal_optional_columns,
    ensure_pending_rename_optional_columns, ensure_samples_optional_columns,
    ensure_wav_files_optional_columns,
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
    Ok(())
}

/// Apply low-cost additive repairs that must not be skipped for current stamps.
pub(super) fn apply_current_stamp_repairs(connection: &Connection) -> Result<(), SourceDbError> {
    ensure_pending_rename_optional_columns(connection)?;
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
