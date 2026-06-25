use std::path::Path;

use rusqlite::{CachedStatement, OptionalExtension, Transaction, params};

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

fn execute_cached_update_expect_one(
    mut statement: CachedStatement<'_>,
    params: impl rusqlite::Params,
) -> Result<(), SourceDbError> {
    let changed = statement.execute(params).map_err(map_sql_error)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(SourceDbError::Unexpected)
    }
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
    execute_cached_update_expect_one(
        tx.prepare_cached(sql).map_err(map_sql_error)?,
        params![value, path],
    )
}

pub(super) fn update_path_text_statement(
    tx: &Transaction<'_>,
    sql: &str,
    relative_path: &Path,
    value: &str,
) -> Result<(), SourceDbError> {
    let path = normalize_relative_path(relative_path)?;
    execute_cached_update_expect_one(
        tx.prepare_cached(sql).map_err(map_sql_error)?,
        params![value, path],
    )
}

pub(super) fn update_path_null_statement(
    tx: &Transaction<'_>,
    sql: &str,
    relative_path: &Path,
) -> Result<(), SourceDbError> {
    let path = normalize_relative_path(relative_path)?;
    execute_cached_update_expect_one(
        tx.prepare_cached(sql).map_err(map_sql_error)?,
        params![path],
    )
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

pub(super) fn remap_wav_file_path_statement(
    tx: &Transaction<'_>,
    old_relative_path: &Path,
    new_relative_path: &Path,
) -> Result<(), SourceDbError> {
    let old_path = normalize_relative_path(old_relative_path)?;
    let new_path = normalize_relative_path(new_relative_path)?;
    if old_path == new_path {
        return Ok(());
    }

    tx.prepare_cached(DELETE_WAV_FILE_SQL)
        .map_err(map_sql_error)?
        .execute(params![new_path.as_str()])
        .map_err(map_sql_error)?;
    let changed = tx
        .prepare_cached(
            "INSERT INTO wav_files (
                 path, file_size, modified_ns, tag, looped, locked, sound_type,
                 user_tag, tag_named, missing, extension, last_played_at, last_curated_at,
                 collection
             )
             SELECT ?2, file_size, modified_ns, tag, looped, locked, sound_type,
                    user_tag, tag_named, missing, extension, last_played_at, last_curated_at,
                    collection
             FROM wav_files
             WHERE path = ?1",
        )
        .map_err(map_sql_error)?
        .execute(params![old_path.as_str(), new_path.as_str()])
        .map_err(map_sql_error)?;
    if changed == 0 {
        return Ok(());
    }

    tx.prepare_cached(
        "INSERT OR IGNORE INTO wav_file_tags (path, tag_id)
         SELECT ?2, tag_id FROM wav_file_tags WHERE path = ?1",
    )
    .map_err(map_sql_error)?
    .execute(params![old_path.as_str(), new_path.as_str()])
    .map_err(map_sql_error)?;
    tx.prepare_cached(
        "INSERT OR IGNORE INTO wav_file_collections (path, collection)
         SELECT ?2, collection FROM wav_file_collections WHERE path = ?1",
    )
    .map_err(map_sql_error)?
    .execute(params![old_path.as_str(), new_path.as_str()])
    .map_err(map_sql_error)?;
    tx.prepare_cached(DELETE_WAV_FILE_SQL)
        .map_err(map_sql_error)?
        .execute(params![old_path])
        .map_err(map_sql_error)?;
    Ok(())
}

pub(super) fn remap_analysis_sample_identity_statement(
    tx: &Transaction<'_>,
    old_relative_path: &Path,
    new_relative_path: &Path,
) -> Result<(), SourceDbError> {
    let old_path = normalize_relative_path(old_relative_path)?;
    let new_path = normalize_relative_path(new_relative_path)?;
    if old_path == new_path {
        return Ok(());
    }
    let old_ids = sample_ids_for_relative_path(tx, &old_path)?;
    for old_sample_id in old_ids {
        let Some(source_prefix) = old_sample_id.strip_suffix(&old_path) else {
            continue;
        };
        if !source_prefix.ends_with("::") {
            continue;
        }
        let new_sample_id = format!("{source_prefix}{new_path}");
        remap_one_analysis_sample_identity(tx, &old_sample_id, &new_sample_id, &new_path)?;
    }
    Ok(())
}

fn sample_ids_for_relative_path(
    tx: &Transaction<'_>,
    relative_path: &str,
) -> Result<Vec<String>, SourceDbError> {
    let mut sample_ids = Vec::new();
    let mut stmt = tx
        .prepare_cached(
            "SELECT sample_id FROM samples
             WHERE substr(sample_id, -length(?1)) = ?1
               AND substr(sample_id, length(sample_id) - length(?1) - 1, 2) = '::'
             UNION
             SELECT sample_id FROM analysis_jobs
             WHERE relative_path = ?2",
        )
        .map_err(map_sql_error)?;
    let rows = stmt
        .query_map(params![relative_path, relative_path], |row| {
            row.get::<_, String>(0)
        })
        .map_err(map_sql_error)?;
    for row in rows {
        sample_ids.push(row.map_err(map_sql_error)?);
    }
    Ok(sample_ids)
}

fn remap_one_analysis_sample_identity(
    tx: &Transaction<'_>,
    old_sample_id: &str,
    new_sample_id: &str,
    new_relative_path: &str,
) -> Result<(), SourceDbError> {
    if sample_row_exists(tx, old_sample_id)? && !sample_row_exists(tx, new_sample_id)? {
        tx.prepare_cached(
            "INSERT INTO samples (
                 sample_id, content_hash, size, mtime_ns, duration_seconds,
                 sr_used, analysis_version, bpm, long_sample_mark
             )
             SELECT ?2, content_hash, size, mtime_ns, duration_seconds,
                    sr_used, analysis_version, bpm, long_sample_mark
             FROM samples
             WHERE sample_id = ?1",
        )
        .map_err(map_sql_error)?
        .execute(params![old_sample_id, new_sample_id])
        .map_err(map_sql_error)?;
    }

    update_sample_id(tx, "analysis_features", old_sample_id, new_sample_id)?;
    update_sample_id(tx, "features", old_sample_id, new_sample_id)?;
    update_sample_id(tx, "embeddings", old_sample_id, new_sample_id)?;
    update_sample_id(tx, "layout_umap", old_sample_id, new_sample_id)?;
    update_sample_id(tx, "hdbscan_clusters", old_sample_id, new_sample_id)?;
    tx.prepare_cached(
        "UPDATE analysis_jobs
         SET sample_id = ?2, relative_path = ?3
         WHERE sample_id = ?1",
    )
    .map_err(map_sql_error)?
    .execute(params![old_sample_id, new_sample_id, new_relative_path])
    .map_err(map_sql_error)?;
    tx.prepare_cached("DELETE FROM samples WHERE sample_id = ?1")
        .map_err(map_sql_error)?
        .execute(params![old_sample_id])
        .map_err(map_sql_error)?;
    Ok(())
}

fn sample_row_exists(tx: &Transaction<'_>, sample_id: &str) -> Result<bool, SourceDbError> {
    let exists = tx
        .query_row(
            "SELECT 1 FROM samples WHERE sample_id = ?1",
            params![sample_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(map_sql_error)?
        .is_some();
    Ok(exists)
}

fn update_sample_id(
    tx: &Transaction<'_>,
    table: &str,
    old_sample_id: &str,
    new_sample_id: &str,
) -> Result<(), SourceDbError> {
    let sql = format!("UPDATE {table} SET sample_id = ?2 WHERE sample_id = ?1");
    tx.prepare_cached(&sql)
        .map_err(map_sql_error)?
        .execute(params![old_sample_id, new_sample_id])
        .map_err(map_sql_error)?;
    Ok(())
}
