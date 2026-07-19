use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension};

use super::super::super::SourceDbError;
use super::super::super::util::map_sql_error;
use super::table_columns;

pub(super) fn ensure_analysis_jobs_optional_columns(
    connection: &Connection,
) -> Result<(), SourceDbError> {
    let columns = table_columns(connection, "analysis_jobs")?;
    if !columns.contains("running_at") {
        connection
            .execute(
                "ALTER TABLE analysis_jobs ADD COLUMN running_at INTEGER",
                [],
            )
            .map_err(map_sql_error)?;
        let now = now_epoch_seconds();
        connection
            .execute(
                "UPDATE analysis_jobs SET running_at = ?1 WHERE status = 'running'",
                [now],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("source_id") {
        connection
            .execute(
                "ALTER TABLE analysis_jobs ADD COLUMN source_id TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(map_sql_error)?;
    }
    backfill_analysis_jobs_source_id(connection)?;
    if !columns.contains("relative_path") {
        connection
            .execute(
                "ALTER TABLE analysis_jobs ADD COLUMN relative_path TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(map_sql_error)?;
    }
    backfill_analysis_jobs_relative_path(connection)?;
    for (column, definition) in [
        ("readiness_managed", "INTEGER NOT NULL DEFAULT 0"),
        ("readiness_claim_generation", "INTEGER NOT NULL DEFAULT 0"),
        ("readiness_scope_kind", "TEXT"),
        ("readiness_scope_id", "TEXT"),
        ("readiness_stage", "TEXT"),
        ("artifact_version", "TEXT"),
        ("source_generation", "INTEGER"),
        ("content_generation", "TEXT"),
        ("retry_at", "INTEGER"),
        ("failure_kind", "TEXT"),
        ("failure_code", "TEXT"),
        ("lease_expires_at", "INTEGER"),
    ] {
        if !columns.contains(column) {
            connection
                .execute(
                    &format!("ALTER TABLE analysis_jobs ADD COLUMN {column} {definition}"),
                    [],
                )
                .map_err(map_sql_error)?;
        }
    }
    Ok(())
}

pub(super) fn ensure_source_readiness_schema(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS source_readiness_sources (
                source_id TEXT PRIMARY KEY,
                source_generation INTEGER NOT NULL,
                readiness_revision INTEGER NOT NULL,
                availability TEXT NOT NULL,
                contract_version TEXT NOT NULL DEFAULT '',
                membership_digest BLOB NOT NULL DEFAULT X'',
                membership_count INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL
            ) WITHOUT ROWID;
            CREATE TABLE IF NOT EXISTS source_readiness_targets (
                source_id TEXT NOT NULL,
                scope_kind TEXT NOT NULL,
                scope_id TEXT NOT NULL,
                relative_path TEXT,
                stage TEXT NOT NULL,
                required_version TEXT NOT NULL CHECK(length(trim(required_version)) > 0),
                source_generation INTEGER NOT NULL,
                content_generation TEXT NOT NULL CHECK(length(trim(content_generation)) > 0),
                eligibility TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                CHECK (
                    (stage = 'similarity_layout' AND scope_kind = 'source')
                    OR (stage <> 'similarity_layout' AND scope_kind = 'file')
                ),
                CHECK (length(trim(source_id)) > 0 AND length(trim(scope_id)) > 0),
                CHECK (
                    (scope_kind = 'source' AND scope_id = source_id AND relative_path IS NULL)
                    OR (
                        scope_kind = 'file'
                        AND (
                            eligibility <> 'eligible'
                            OR (relative_path IS NOT NULL AND length(trim(relative_path)) > 0)
                        )
                    )
                ),
                PRIMARY KEY (source_id, scope_kind, scope_id, stage)
            ) WITHOUT ROWID;
            CREATE TABLE IF NOT EXISTS source_readiness_artifacts (
                source_id TEXT NOT NULL,
                scope_kind TEXT NOT NULL,
                scope_id TEXT NOT NULL,
                relative_path TEXT,
                stage TEXT NOT NULL,
                artifact_version TEXT NOT NULL CHECK(length(trim(artifact_version)) > 0),
                source_generation INTEGER NOT NULL,
                content_generation TEXT NOT NULL CHECK(length(trim(content_generation)) > 0),
                artifact_ref TEXT,
                completed_at INTEGER NOT NULL,
                CHECK (
                    (stage = 'similarity_layout' AND scope_kind = 'source')
                    OR (stage <> 'similarity_layout' AND scope_kind = 'file')
                ),
                CHECK (length(trim(source_id)) > 0 AND length(trim(scope_id)) > 0),
                CHECK (scope_kind = 'file' OR (scope_kind = 'source' AND scope_id = source_id)),
                PRIMARY KEY (source_id, scope_kind, scope_id, stage)
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_analysis_jobs_readiness_state
                ON analysis_jobs (
                    source_id,
                    readiness_managed,
                    readiness_stage,
                    status,
                    retry_at
                );
            CREATE INDEX IF NOT EXISTS idx_source_readiness_targets_generation
                ON source_readiness_targets (source_id, source_generation, stage);
            CREATE INDEX IF NOT EXISTS idx_source_readiness_artifacts_generation
                ON source_readiness_artifacts (source_id, source_generation, stage);",
        )
        .map_err(map_sql_error)?;
    let source_columns = table_columns(connection, "source_readiness_sources")?;
    if !source_columns.contains("readiness_revision") {
        connection
            .execute(
                "ALTER TABLE source_readiness_sources
                 ADD COLUMN readiness_revision INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(map_sql_error)?;
    }
    for (column, definition) in [
        ("contract_version", "TEXT NOT NULL DEFAULT ''"),
        ("membership_digest", "BLOB NOT NULL DEFAULT X''"),
        ("membership_count", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        if !source_columns.contains(column) {
            connection
                .execute(
                    &format!(
                        "ALTER TABLE source_readiness_sources ADD COLUMN {column} {definition}"
                    ),
                    [],
                )
                .map_err(map_sql_error)?;
        }
    }
    let artifact_columns = table_columns(connection, "source_readiness_artifacts")?;
    for (column, definition) in [("relative_path", "TEXT"), ("artifact_ref", "TEXT")] {
        if !artifact_columns.contains(column) {
            connection
                .execute(
                    &format!(
                        "ALTER TABLE source_readiness_artifacts ADD COLUMN {column} {definition}"
                    ),
                    [],
                )
                .map_err(map_sql_error)?;
        }
    }
    Ok(())
}

/// Retire rows owned by the pre-readiness controller runtime exactly once as
/// part of the v9 schema upgrade.
///
/// The transaction keeps the migration restart-safe. The readiness predicate
/// and explicit job-type allowlist preserve current work and unknown producers.
pub(super) fn retire_legacy_analysis_runtime_state(
    connection: &Connection,
) -> Result<(), SourceDbError> {
    let tx = connection.unchecked_transaction().map_err(map_sql_error)?;
    tx.execute(
        "DELETE FROM analysis_jobs
         WHERE readiness_managed = 0
           AND job_type IN (
               'wav_metadata_v1',
               'embedding_backfill_v1',
               'rebuild_index_v1'
           )",
        [],
    )
    .map_err(map_sql_error)?;
    let progress_table_exists = tx
        .query_row(
            "SELECT 1 FROM sqlite_master
             WHERE type = 'table' AND name = 'analysis_job_progress_snapshots'",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(map_sql_error)?
        .is_some();
    if progress_table_exists {
        tx.execute(
            "DELETE FROM analysis_job_progress_snapshots
             WHERE job_type IN (
                 'wav_metadata_v1',
                 'embedding_backfill_v1',
                 'rebuild_index_v1'
             )",
            [],
        )
        .map_err(map_sql_error)?;
    }
    tx.commit().map_err(map_sql_error)?;
    Ok(())
}

fn backfill_analysis_jobs_source_id(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .execute(
            "UPDATE analysis_jobs
             SET source_id = CASE
                 WHEN instr(sample_id, '::') > 0
                 THEN substr(sample_id, 1, instr(sample_id, '::') - 1)
                 ELSE source_id
             END
             WHERE source_id = '' OR source_id IS NULL",
            [],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn backfill_analysis_jobs_relative_path(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .execute(
            "UPDATE analysis_jobs
             SET relative_path = CASE
                 WHEN instr(sample_id, '::') > 0
                 THEN substr(sample_id, instr(sample_id, '::') + 2)
                 ELSE relative_path
             END
             WHERE relative_path = '' OR relative_path IS NULL",
            [],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}
