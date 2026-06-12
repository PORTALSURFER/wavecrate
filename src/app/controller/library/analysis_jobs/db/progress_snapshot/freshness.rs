use crate::sample_sources::db::META_WAV_PATHS_REVISION;
use rusqlite::{Connection, OptionalExtension, params};

const ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY: &str =
    "analysis_progress_snapshot_wav_paths_revision_v1";

pub(super) fn analyze_snapshot_is_fresh(conn: &Connection) -> Result<bool, String> {
    Ok(read_analyze_snapshot_wav_paths_revision(conn)? == current_wav_paths_revision(conn)?)
}

pub(super) fn store_analyze_snapshot_wav_paths_revision(conn: &Connection) -> Result<(), String> {
    let Some(revision) = current_wav_paths_revision(conn)? else {
        conn.execute(
            "DELETE FROM metadata WHERE key = ?1",
            params![ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY],
        )
        .map_err(|err| err.to_string())?;
        return Ok(());
    };
    conn.execute(
        "INSERT INTO metadata (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY, revision],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn read_analyze_snapshot_wav_paths_revision(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| err.to_string())
}

fn current_wav_paths_revision(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![META_WAV_PATHS_REVISION],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| err.to_string())
}
