use super::*;
use fixtures::{column_names, with_legacy_db};

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

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
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

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
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
