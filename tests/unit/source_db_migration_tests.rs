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
fn wav_files_migration_backfills_sound_type_and_user_tag_into_normal_tags() {
    let dir = with_legacy_db(
        "CREATE TABLE wav_files (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            sound_type TEXT,
            user_tag TEXT,
            looped INTEGER NOT NULL DEFAULT 0
        );
        INSERT INTO wav_files (path, file_size, modified_ns, sound_type, user_tag, looped)
        VALUES
            ('one.wav', 10, 5, 'kick', 'Deep   Kick', 1),
            ('two.wav', 10, 5, 'KICK', 'deep kick', 0),
            ('loop.wav', 10, 5, NULL, '', 1);",
    );

    let db = SourceDatabase::open(dir.path()).unwrap();
    let tag_columns = column_names(&db.connection, "source_tags");
    let assignment_columns = column_names(&db.connection, "wav_file_tags");
    assert!(tag_columns.iter().any(|column| column == "normalized_text"));
    assert!(tag_columns.iter().any(|column| column == "display_label"));
    assert!(assignment_columns.iter().any(|column| column == "path"));
    assert!(assignment_columns.iter().any(|column| column == "tag_id"));

    let labels = db
        .most_used_tags(8)
        .unwrap()
        .into_iter()
        .map(|usage| {
            (
                usage.tag.display_label,
                usage.tag.normalized_text,
                usage.assignment_count,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            ("Deep Kick".to_string(), "deep kick".to_string(), 2),
            ("kick".to_string(), "kick".to_string(), 2),
        ]
    );
    assert!(
        db.tags_for_path(std::path::Path::new("loop.wav"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.looped_for_path(std::path::Path::new("one.wav")).unwrap(),
        Some(true)
    );
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

#[test]
fn pending_rename_migration_adds_metadata_columns_and_keeps_legacy_rows_readable() {
    let dir = with_legacy_db(
        "CREATE TABLE pending_wav_renames (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            content_hash TEXT,
            tag INTEGER NOT NULL,
            looped INTEGER NOT NULL,
            locked INTEGER NOT NULL,
            last_played_at INTEGER
        );
        INSERT INTO pending_wav_renames (
            path, file_size, modified_ns, content_hash, tag, looped, locked, last_played_at
        ) VALUES (
            'legacy.wav', 10, 5, 'hash-a', 1, 1, 1, 42
        );",
    );

    let db = SourceDatabase::open(dir.path()).unwrap();
    let columns = column_names(&db.connection, "pending_wav_renames");
    assert!(columns.iter().any(|column| column == "sound_type"));
    assert!(columns.iter().any(|column| column == "user_tag"));

    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].relative_path, std::path::Path::new("legacy.wav"));
    assert_eq!(pending[0].tag, Rating::KEEP_1);
    assert!(pending[0].looped);
    assert!(pending[0].locked);
    assert_eq!(pending[0].last_played_at, Some(42));
    assert_eq!(pending[0].sound_type, None);
    assert_eq!(pending[0].user_tag, None);
}

#[test]
fn current_stamped_pending_rename_table_repairs_missing_metadata_columns() {
    let dir = with_legacy_db(&format!(
        "CREATE TABLE wav_files (
            path TEXT PRIMARY KEY
        );
        CREATE TABLE pending_wav_renames (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            content_hash TEXT,
            tag INTEGER NOT NULL,
            looped INTEGER NOT NULL,
            locked INTEGER NOT NULL,
            last_played_at INTEGER
        );
        INSERT INTO pending_wav_renames (
            path, file_size, modified_ns, content_hash, tag, looped, locked, last_played_at
        ) VALUES (
            'current-stamped.wav', 10, 5, 'hash-a', 1, 1, 1, 42
        );
        PRAGMA user_version = {};",
        schema::SOURCE_DB_SCHEMA_VERSION
    ));

    let db = SourceDatabase::open(dir.path()).unwrap();
    let columns = column_names(&db.connection, "pending_wav_renames");
    assert!(columns.iter().any(|column| column == "sound_type"));
    assert!(columns.iter().any(|column| column == "user_tag"));

    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].relative_path,
        std::path::Path::new("current-stamped.wav")
    );
    assert_eq!(pending[0].sound_type, None);
    assert_eq!(pending[0].user_tag, None);
}

#[test]
fn current_stamped_wav_files_table_repairs_missing_collection_column() {
    let dir = with_legacy_db(&format!(
        "CREATE TABLE metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE wav_files (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            content_hash TEXT,
            tag INTEGER NOT NULL DEFAULT 0,
            looped INTEGER NOT NULL DEFAULT 0,
            locked INTEGER NOT NULL DEFAULT 0,
            missing INTEGER NOT NULL DEFAULT 0,
            extension TEXT NOT NULL DEFAULT '',
            sound_type TEXT,
            user_tag TEXT,
            tag_named INTEGER NOT NULL DEFAULT 0,
            last_played_at INTEGER
        );
        INSERT INTO wav_files (path, file_size, modified_ns, extension)
        VALUES ('one.wav', 10, 5, 'wav');
        PRAGMA user_version = {};",
        schema::SOURCE_DB_SCHEMA_VERSION
    ));

    let db = SourceDatabase::open_for_user_metadata_write(dir.path()).unwrap();
    let columns = column_names(&db.connection, "wav_files");
    assert!(columns.iter().any(|column| column == "collection"));

    let mut batch = db.write_batch().unwrap();
    batch
        .set_collection(std::path::Path::new("one.wav"), SampleCollection::new(2))
        .unwrap();
    batch.commit().unwrap();

    assert_eq!(
        db.collection_for_path(std::path::Path::new("one.wav"))
            .unwrap(),
        SampleCollection::new(2)
    );
}
