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
