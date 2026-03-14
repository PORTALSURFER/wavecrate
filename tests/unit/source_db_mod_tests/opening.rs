use super::*;

#[test]
fn read_only_open_reads_existing_entries() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let read_only = SourceDatabase::open_read_only(dir.path()).unwrap();
    let rows = read_only.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
}

#[test]
fn open_defaults_to_read_only_when_enabled() {
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("tempdir failed: {err}"),
    };

    assert!(matches!(
        open_source_database(dir.path(), true, false, SourceDatabaseOpenMode::Full),
        Err(SourceDbError::ReadOnlyDatabaseMissing(_))
    ));
}

#[test]
fn open_blocks_writes_for_user_library_roots_without_override() {
    let home = match tempdir() {
        Ok(home) => home,
        Err(err) => panic!("tempdir failed: {err}"),
    };
    let user_home = home.path().join("home");
    let user_music = user_home.join("Music");
    if let Err(err) = std::fs::create_dir_all(&user_music) {
        panic!("create fake user library dir failed: {err}");
    }
    with_home_env_override(&user_home, || {
        let blocked = open_source_database(&user_music, false, false, SourceDatabaseOpenMode::Full);
        assert!(matches!(
            blocked,
            Err(SourceDbError::UserLibraryWriteBlocked { .. })
        ));

        let db = open_source_database(&user_music, false, true, SourceDatabaseOpenMode::Full);
        assert!(db.is_ok());
        let opened = match db {
            Ok(opened) => opened,
            Err(err) => panic!("db open with override should be allowed: {err}"),
        };
        assert_eq!(opened.root(), user_music.as_path());
    });
}

#[test]
fn absolute_paths_are_rejected() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let absolute = std::env::current_dir().unwrap().join("absolute.wav");
    let err = db.upsert_file(&absolute, 1, 1).unwrap_err();
    assert!(matches!(err, SourceDbError::PathMustBeRelative(_)));
}

#[test]
fn parent_dir_paths_are_rejected() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let err = db
        .upsert_file(Path::new("../escape.wav"), 1, 1)
        .unwrap_err();
    assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));
}

#[test]
fn list_files_skips_invalid_relative_paths() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.connection
        .execute(
            "INSERT INTO wav_files (path, file_size, modified_ns, tag, looped, missing, extension)
                 VALUES (?1, ?2, ?3, 0, 0, 0, 'wav')",
            params!["../escape.wav", 1i64, 1i64],
        )
        .unwrap();
    let rows = db.list_files().unwrap();
    assert!(rows.is_empty());
}

#[test]
fn open_removes_invalid_relative_paths() {
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
            "INSERT INTO wav_files (path, file_size, modified_ns) VALUES (?1, ?2, ?3)",
            params!["../escape.wav", 1i64, 1i64],
        )
        .unwrap();
    }
    let db = SourceDatabase::open(dir.path()).unwrap();
    let count: i64 = db
        .connection
        .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn open_fast_defers_invalid_relative_path_cleanup() {
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
            "INSERT INTO wav_files (path, file_size, modified_ns) VALUES (?1, ?2, ?3)",
            params!["../escape.wav", 1i64, 1i64],
        )
        .unwrap();
    }

    let fast = SourceDatabase::open_fast(dir.path()).unwrap();
    let fast_count: i64 = fast
        .connection
        .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fast_count, 1);
    drop(fast);

    let full = SourceDatabase::open(dir.path()).unwrap();
    let full_count: i64 = full
        .connection
        .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(full_count, 0);
}

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
    let _db = SourceDatabase::open(dir.path()).unwrap();
    let conn = Connection::open(dir.path().join(DB_FILE_NAME)).unwrap();

    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .unwrap();
    assert_eq!(synchronous, 2, "expected PRAGMA synchronous=NORMAL (2)");

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
}
