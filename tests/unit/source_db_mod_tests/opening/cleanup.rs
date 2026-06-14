use super::*;

#[test]
fn job_worker_role_defers_invalid_relative_path_cleanup() {
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

    let worker =
        SourceDatabase::open_with_role(dir.path(), SourceDatabaseConnectionRole::JobWorker)
            .unwrap();
    let count: i64 = worker
        .connection
        .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn maintenance_role_cleans_invalid_relative_paths() {
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

    let maintenance =
        SourceDatabase::open_with_role(dir.path(), SourceDatabaseConnectionRole::Maintenance)
            .unwrap();
    let count: i64 = maintenance
        .connection
        .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn absolute_paths_are_rejected() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let absolute = std::env::current_dir().unwrap().join("absolute.wav");
    let err = db.upsert_file(&absolute, 1, 1).unwrap_err();
    #[cfg(windows)]
    assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));
    #[cfg(not(windows))]
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
