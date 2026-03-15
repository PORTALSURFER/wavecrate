use std::path::Path;

use rusqlite::{CachedStatement, Transaction, params};

use crate::sample_sources::SourceDbError;
use crate::sample_sources::db::util::{map_sql_error, normalize_relative_path};

const DELETE_WAV_FILE_SQL: &str = "DELETE FROM wav_files WHERE path = ?1";

fn execute_cached_statement(
    mut statement: CachedStatement<'_>,
    params: impl rusqlite::Params,
) -> Result<(), SourceDbError> {
    statement.execute(params).map_err(map_sql_error)?;
    Ok(())
}

pub(super) fn execute_transaction_cached(
    tx: &Transaction<'_>,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<(), SourceDbError> {
    execute_cached_statement(tx.prepare_cached(sql).map_err(map_sql_error)?, params)
}

pub(super) fn update_flag_statement(
    tx: &Transaction<'_>,
    sql: &str,
    relative_path: &Path,
    value: bool,
) -> Result<(), SourceDbError> {
    update_path_i64_statement(tx, sql, relative_path, value as i64)
}

pub(super) fn update_path_i64_statement(
    tx: &Transaction<'_>,
    sql: &str,
    relative_path: &Path,
    value: i64,
) -> Result<(), SourceDbError> {
    let path = normalize_relative_path(relative_path)?;
    tx.prepare_cached(sql)
        .map_err(map_sql_error)?
        .execute(params![value, path])
        .map_err(map_sql_error)?;
    Ok(())
}

pub(super) fn delete_path_statement(
    tx: &Transaction<'_>,
    relative_path: &Path,
) -> Result<(), SourceDbError> {
    let path = normalize_relative_path(relative_path)?;
    tx.prepare_cached(DELETE_WAV_FILE_SQL)
        .map_err(map_sql_error)?
        .execute(params![path])
        .map_err(map_sql_error)?;
    Ok(())
}
