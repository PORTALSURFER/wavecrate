use rusqlite::Connection;

pub(crate) fn has_pending_or_running_jobs(conn: &Connection) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM analysis_jobs
            WHERE readiness_managed = 1
              AND status IN ('pending', 'running')
        )",
        [],
        |row| row.get(0),
    )
    .map_err(|err| format!("Failed to inspect readiness jobs: {err}"))
}
