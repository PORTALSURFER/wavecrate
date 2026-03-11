use super::*;
use rusqlite::Connection;
use tempfile::tempdir;

fn with_legacy_db(setup_sql: &str) -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let db_file = dir.path().join(DB_FILE_NAME);
    let conn = Connection::open(&db_file).unwrap();
    conn.execute_batch(setup_sql).unwrap();
    dir
}

fn column_names(connection: &Connection, table_name: &str) -> Vec<String> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = connection.prepare(&pragma).unwrap();
    stmt.query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn wav_files_migration_adds_optional_columns_and_backfills_extension() {
    let dir = with_legacy_db(
        "CREATE TABLE wav_files (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL
        );
        INSERT INTO wav_files (path, file_size, modified_ns)
        VALUES ('nested/One.WAV', 10, 5);",
    );

    let db = SourceDatabase::open(dir.path()).unwrap();
    let columns = column_names(&db.connection, "wav_files");
    assert!(columns.iter().any(|column| column == "tag"));
    assert!(columns.iter().any(|column| column == "looped"));
    assert!(columns.iter().any(|column| column == "locked"));
    assert!(columns.iter().any(|column| column == "missing"));
    assert!(columns.iter().any(|column| column == "extension"));
    assert!(columns.iter().any(|column| column == "last_played_at"));

    let row = db
        .connection
        .query_row(
            "SELECT tag, looped, locked, missing, extension, last_played_at
             FROM wav_files
             WHERE path = 'nested/One.WAV'",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(row.0, 0);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, "wav");
    assert_eq!(row.5, None);
}

#[test]
fn analysis_jobs_migration_backfills_running_rows_and_sample_parts() {
    let dir = with_legacy_db(
        "CREATE TABLE analysis_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sample_id TEXT NOT NULL,
            job_type TEXT NOT NULL,
            content_hash TEXT,
            status TEXT NOT NULL,
            attempts INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            last_error TEXT,
            UNIQUE(sample_id, job_type)
        );
        INSERT INTO analysis_jobs (
            sample_id, job_type, content_hash, status, attempts, created_at, last_error
        )
        VALUES ('source-a::folder/one.wav', 'backfill', 'hash-a', 'running', 1, 10, NULL);",
    );

    let db = SourceDatabase::open(dir.path()).unwrap();
    let columns = column_names(&db.connection, "analysis_jobs");
    assert!(columns.iter().any(|column| column == "running_at"));
    assert!(columns.iter().any(|column| column == "source_id"));
    assert!(columns.iter().any(|column| column == "relative_path"));

    let row = db
        .connection
        .query_row(
            "SELECT source_id, relative_path, running_at
             FROM analysis_jobs
             WHERE sample_id = 'source-a::folder/one.wav'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(row.0, "source-a");
    assert_eq!(row.1, "folder/one.wav");
    assert!(row.2.is_some());
}

#[test]
fn samples_migration_adds_optional_analysis_columns() {
    let dir = with_legacy_db(
        "CREATE TABLE samples (
            sample_id TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL,
            size INTEGER NOT NULL,
            mtime_ns INTEGER NOT NULL
        );
        INSERT INTO samples (sample_id, content_hash, size, mtime_ns)
        VALUES ('source-a::one.wav', 'hash-a', 10, 5);",
    );

    let db = SourceDatabase::open(dir.path()).unwrap();
    let columns = column_names(&db.connection, "samples");
    assert!(columns.iter().any(|column| column == "duration_seconds"));
    assert!(columns.iter().any(|column| column == "sr_used"));
    assert!(columns.iter().any(|column| column == "analysis_version"));
    assert!(columns.iter().any(|column| column == "bpm"));
    assert!(columns.iter().any(|column| column == "long_sample_mark"));

    let row = db
        .connection
        .query_row(
            "SELECT duration_seconds, sr_used, analysis_version, bpm, long_sample_mark
             FROM samples
             WHERE sample_id = 'source-a::one.wav'",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<f64>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<f64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(row.0, None);
    assert_eq!(row.1, None);
    assert_eq!(row.2, None);
    assert_eq!(row.3, None);
    assert_eq!(row.4, None);
}

#[test]
fn analysis_jobs_migration_preserves_rows_without_source_separator() {
    let dir = with_legacy_db(
        "CREATE TABLE analysis_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sample_id TEXT NOT NULL,
            job_type TEXT NOT NULL,
            content_hash TEXT,
            status TEXT NOT NULL,
            attempts INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            last_error TEXT,
            UNIQUE(sample_id, job_type)
        );
        INSERT INTO analysis_jobs (
            sample_id, job_type, content_hash, status, attempts, created_at, last_error
        )
        VALUES ('orphan-id', 'backfill', 'hash-a', 'pending', 0, 10, NULL);",
    );

    let db = SourceDatabase::open(dir.path()).unwrap();
    let row = db
        .connection
        .query_row(
            "SELECT source_id, relative_path, running_at
             FROM analysis_jobs
             WHERE sample_id = 'orphan-id'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(row.0, "");
    assert_eq!(row.1, "");
    assert_eq!(row.2, None);
}
