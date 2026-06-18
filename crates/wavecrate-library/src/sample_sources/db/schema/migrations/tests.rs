use super::*;

#[test]
fn analysis_jobs_backfill_blank_identity_columns() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE analysis_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sample_id TEXT NOT NULL,
            source_id TEXT NOT NULL DEFAULT '',
            relative_path TEXT NOT NULL DEFAULT '',
            job_type TEXT NOT NULL,
            status TEXT NOT NULL,
            attempts INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );
        INSERT INTO analysis_jobs (
            sample_id, source_id, relative_path, job_type, status, attempts, created_at
        ) VALUES (
            'source-a::Pack/a.wav', '', '', 'analyze_sample', 'pending', 0, 0
        );",
    )
    .unwrap();

    ensure_analysis_jobs_optional_columns(&conn).unwrap();

    let row: (String, String) = conn
        .query_row(
            "SELECT source_id, relative_path FROM analysis_jobs",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(row.0, "source-a");
    assert_eq!(row.1, "Pack/a.wav");
}

#[test]
fn pending_rename_migration_adds_extended_metadata_columns() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE pending_wav_renames (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            content_hash TEXT,
            tag INTEGER NOT NULL,
            looped INTEGER NOT NULL,
            locked INTEGER NOT NULL,
            last_played_at INTEGER
        );",
    )
    .unwrap();

    ensure_pending_rename_optional_columns(&conn).unwrap();

    let columns = table_columns(&conn, "pending_wav_renames").unwrap();
    assert!(columns.contains("sound_type"));
    assert!(columns.contains("user_tag"));
    assert!(columns.contains("normal_tags"));
}

#[test]
fn collection_membership_schema_backfills_legacy_collection_column() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE wav_files (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            collection INTEGER
        );
        INSERT INTO wav_files (path, file_size, modified_ns, collection)
        VALUES ('nested/One.WAV', 10, 5, 2);",
    )
    .unwrap();

    ensure_collection_membership_schema(&conn).unwrap();

    let collection: i64 = conn
        .query_row(
            "SELECT collection
             FROM wav_file_collections
             WHERE path = 'nested/One.WAV'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(collection, 2);
}

#[test]
fn aspect_descriptor_schema_creates_current_and_cache_tables() {
    let conn = Connection::open_in_memory().unwrap();

    ensure_aspect_descriptor_tables(&conn).unwrap();

    let current_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM sqlite_master
             WHERE type = 'table' AND name = 'similarity_aspect_descriptors'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cache_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM sqlite_master
             WHERE type = 'table' AND name = 'analysis_cache_aspect_descriptors'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(current_exists, 1);
    assert_eq!(cache_exists, 1);
}
