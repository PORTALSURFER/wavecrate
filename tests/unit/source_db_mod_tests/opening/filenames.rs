use super::*;

#[test]
fn writable_open_uses_current_source_database_filename() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    assert_eq!(db.db_path, dir.path().join(DB_FILE_NAME));
    assert!(dir.path().join(DB_FILE_NAME).is_file());
    assert!(!dir.path().join(LEGACY_DB_FILE_NAME).exists());
}

#[test]
fn writable_open_migrates_legacy_source_database_filename() {
    let dir = tempdir().unwrap();
    let legacy_db = dir.path().join(LEGACY_DB_FILE_NAME);
    {
        let conn = Connection::open(&legacy_db).unwrap();
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

    assert_eq!(db.db_path, dir.path().join(DB_FILE_NAME));
    assert!(dir.path().join(DB_FILE_NAME).is_file());
    assert!(!legacy_db.exists());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
}

#[test]
fn read_only_open_keeps_legacy_source_database_filename() {
    let dir = tempdir().unwrap();
    let legacy_db = dir.path().join(LEGACY_DB_FILE_NAME);
    {
        let conn = Connection::open(&legacy_db).unwrap();
        conn.execute(
            "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                )",
            [],
        )
        .unwrap();
    }

    let db = SourceDatabase::open_read_only(dir.path()).unwrap();

    assert_eq!(db.db_path, legacy_db);
    assert!(dir.path().join(LEGACY_DB_FILE_NAME).is_file());
    assert!(!dir.path().join(DB_FILE_NAME).exists());
}

#[cfg(unix)]
#[test]
fn writable_open_rejects_symlinked_current_source_database_before_sqlite_open() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_db = outside.path().join("outside.db");
    Connection::open(&outside_db).unwrap();
    let db_path = dir.path().join(DB_FILE_NAME);
    std::os::unix::fs::symlink(&outside_db, &db_path).unwrap();

    expect_unsafe_source_db_path(SourceDatabase::open(dir.path()), &db_path);

    assert!(
        !sqlite_table_exists(&outside_db, "metadata"),
        "outside database must not receive Wavecrate schema"
    );
}

#[cfg(unix)]
#[test]
fn read_only_open_rejects_symlinked_current_source_database_explicitly() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_db = outside.path().join("outside.db");
    Connection::open(&outside_db).unwrap();
    let db_path = dir.path().join(DB_FILE_NAME);
    std::os::unix::fs::symlink(&outside_db, &db_path).unwrap();

    expect_unsafe_source_db_path(SourceDatabase::open_read_only(dir.path()), &db_path);
}

#[cfg(unix)]
#[test]
fn writable_open_rejects_symlinked_legacy_source_database_before_migration() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let outside_db = outside.path().join("outside.db");
    Connection::open(&outside_db).unwrap();
    let legacy_db = dir.path().join(LEGACY_DB_FILE_NAME);
    std::os::unix::fs::symlink(&outside_db, &legacy_db).unwrap();

    expect_unsafe_source_db_path(SourceDatabase::open(dir.path()), &legacy_db);

    assert!(!dir.path().join(DB_FILE_NAME).exists());
    assert!(
        !sqlite_table_exists(&outside_db, "metadata"),
        "outside database must not receive Wavecrate schema"
    );
}

#[cfg(unix)]
#[test]
fn writable_open_rejects_symlinked_current_wal_and_shm_sidecars_before_sqlite_open() {
    for suffix in ["-wal", "-shm"] {
        let dir = tempdir().unwrap();
        SourceDatabase::open(dir.path()).unwrap();
        let sidecar = sqlite_sidecar_path(&dir.path().join(DB_FILE_NAME), suffix);
        let outside = tempdir().unwrap();
        let outside_sidecar = outside.path().join("outside-sidecar");
        std::fs::write(&outside_sidecar, b"outside").unwrap();
        std::os::unix::fs::symlink(&outside_sidecar, &sidecar).unwrap();

        expect_unsafe_source_db_path(SourceDatabase::open(dir.path()), &sidecar);

        assert_eq!(std::fs::read(&outside_sidecar).unwrap(), b"outside");
    }
}

#[cfg(unix)]
#[test]
fn writable_open_rejects_symlinked_legacy_wal_and_shm_sidecars_before_rename() {
    for suffix in ["-wal", "-shm"] {
        let dir = tempdir().unwrap();
        let legacy_db = dir.path().join(LEGACY_DB_FILE_NAME);
        {
            let conn = Connection::open(&legacy_db).unwrap();
            conn.execute(
                "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();
        }
        let sidecar = sqlite_sidecar_path(&legacy_db, suffix);
        let outside = tempdir().unwrap();
        let outside_sidecar = outside.path().join("outside-sidecar");
        std::fs::write(&outside_sidecar, b"outside").unwrap();
        std::os::unix::fs::symlink(&outside_sidecar, &sidecar).unwrap();

        expect_unsafe_source_db_path(SourceDatabase::open(dir.path()), &sidecar);

        assert!(
            legacy_db.exists(),
            "legacy DB should not be renamed after sidecar rejection"
        );
        assert!(!dir.path().join(DB_FILE_NAME).exists());
        assert_eq!(std::fs::read(&outside_sidecar).unwrap(), b"outside");
    }
}

#[cfg(unix)]
fn expect_unsafe_source_db_path(result: Result<SourceDatabase, SourceDbError>, expected: &Path) {
    match result {
        Err(SourceDbError::UnsafeSourceDatabasePath { path, reason }) => {
            assert_eq!(path, expected);
            assert!(
                reason.contains("symlink"),
                "unexpected unsafe-path reason: {reason}"
            );
        }
        Err(err) => panic!("expected unsafe source DB path error for {expected:?}, got {err}"),
        Ok(_) => panic!("expected unsafe source DB path error for {expected:?}"),
    }
}

#[cfg(unix)]
fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = std::ffi::OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(unix)]
fn sqlite_table_exists(db_path: &Path, table: &str) -> bool {
    let conn = Connection::open(db_path).unwrap();
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .unwrap()
    .is_some()
}
