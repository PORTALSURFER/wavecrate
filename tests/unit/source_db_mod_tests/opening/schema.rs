use super::*;

#[test]
fn missing_columns_are_added_on_open() {
    let dir = tempdir().unwrap();
    let db_file = dir.path().join(DB_FILE_NAME);
    {
        let conn = Connection::open(&db_file).unwrap();
        conn.execute(
            "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO wav_files (path, file_size, modified_ns) VALUES ('one.wav', 10, 5)",
            [],
        )
        .unwrap();
    }
    let db = SourceDatabase::open(dir.path()).unwrap();
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tag, Rating::NEUTRAL);
    assert!(!rows[0].missing);
}

#[test]
fn applies_workload_pragmas_and_indices() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let conn = &db.connection;

    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .unwrap();
    assert_eq!(synchronous, 1, "expected PRAGMA synchronous=NORMAL (1)");

    let wal_autocheckpoint: i64 = conn
        .query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))
        .unwrap();
    assert_eq!(wal_autocheckpoint, 4096);

    let journal_size_limit: i64 = conn
        .query_row("PRAGMA journal_size_limit", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_size_limit, 67_108_864);

    let busy_timeout: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();
    assert_eq!(busy_timeout, 5000);

    let idx: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_wav_files_missing'",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert_eq!(idx.as_deref(), Some("idx_wav_files_missing"));
    assert_eq!(schema_version(conn), current_schema_version());
}

#[test]
fn stale_schema_stamp_reassures_legacy_files_on_open() {
    let dir = tempdir().unwrap();
    let db_file = dir.path().join(DB_FILE_NAME);
    let conn = Connection::open(&db_file).unwrap();
    conn.execute_batch(
        "CREATE TABLE wav_files (
             path TEXT PRIMARY KEY,
             file_size INTEGER NOT NULL,
             modified_ns INTEGER NOT NULL
         );
         PRAGMA user_version = 0;",
    )
    .unwrap();
    drop(conn);

    let reopened = SourceDatabase::open(dir.path()).unwrap();
    assert_eq!(
        schema_version(&reopened.connection),
        current_schema_version()
    );
}

#[test]
fn ui_read_queries_remain_available_while_job_worker_holds_write_transaction() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    drop(db);

    let mut worker = SourceDatabase::open_connection_with_role(
        dir.path(),
        SourceDatabaseConnectionRole::JobWorker,
    )
    .unwrap();
    let tx = worker
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    tx.execute(
        "UPDATE wav_files SET file_size = file_size + 1 WHERE path = 'one.wav'",
        [],
    )
    .unwrap();

    let ui_read =
        SourceDatabase::open_with_role(dir.path(), SourceDatabaseConnectionRole::UiRead).unwrap();
    let count = ui_read.list_files().unwrap().len();
    assert_eq!(count, 1);

    tx.rollback().unwrap();
}

#[test]
fn ui_read_role_uses_short_busy_timeout() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    drop(db);

    let ui_read =
        SourceDatabase::open_with_role(dir.path(), SourceDatabaseConnectionRole::UiRead).unwrap();
    let busy_timeout: i64 = ui_read
        .connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();
    assert_eq!(busy_timeout, 25);
}
