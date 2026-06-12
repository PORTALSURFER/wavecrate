pub(super) const SNAPSHOT_SCHEMA_SQL: &str =
    "CREATE TABLE IF NOT EXISTS analysis_job_progress_snapshots (
        job_type TEXT PRIMARY KEY,
        pending INTEGER NOT NULL DEFAULT 0,
        running INTEGER NOT NULL DEFAULT 0,
        done INTEGER NOT NULL DEFAULT 0,
        failed INTEGER NOT NULL DEFAULT 0
    ) WITHOUT ROWID;";

pub(super) fn ensure_snapshot_schema(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(SNAPSHOT_SCHEMA_SQL)
        .map_err(|err| err.to_string())
}
