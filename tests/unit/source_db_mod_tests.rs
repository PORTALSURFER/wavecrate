use super::*;
use rusqlite::{OptionalExtension, params};
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};
use tempfile::tempdir;

fn with_home_env_override<T>(home: &Path, test: impl FnOnce() -> T) -> T {
    static HOME_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _lock = match HOME_ENV_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(lock) => lock,
        Err(_) => panic!("HOME env override lock was poisoned"),
    };
    let prev_home = std::env::var_os("HOME");
    let prev_homedrive = std::env::var_os("HOMEDRIVE");
    let prev_hompath = std::env::var_os("HOMEPATH");
    let prev_user_profile = std::env::var_os("USERPROFILE");

    unsafe {
        std::env::set_var("HOME", home);
    }

    struct HomeEnvGuard {
        prev_home: Option<OsString>,
        prev_homedrive: Option<OsString>,
        prev_hompath: Option<OsString>,
        prev_user_profile: Option<OsString>,
    }

    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            match self.prev_home.take() {
                Some(home) => unsafe { std::env::set_var("HOME", home) },
                None => unsafe { std::env::remove_var("HOME") },
            }
            match self.prev_homedrive.take() {
                Some(value) => unsafe { std::env::set_var("HOMEDRIVE", value) },
                None => unsafe { std::env::remove_var("HOMEDRIVE") },
            }
            match self.prev_hompath.take() {
                Some(value) => unsafe { std::env::set_var("HOMEPATH", value) },
                None => unsafe { std::env::remove_var("HOMEPATH") },
            }
            match self.prev_user_profile.take() {
                Some(value) => unsafe { std::env::set_var("USERPROFILE", value) },
                None => unsafe { std::env::remove_var("USERPROFILE") },
            }
        }
    }

    let _home_guard = HomeEnvGuard {
        prev_home,
        prev_homedrive,
        prev_hompath,
        prev_user_profile,
    };

    test()
}

#[test]
fn tags_default_and_persist() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let first = db.list_files().unwrap();
    assert_eq!(first[0].tag, Rating::NEUTRAL);
    assert!(!first[0].looped);
    assert!(!first[0].missing);

    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    let second = db.list_files().unwrap();
    assert_eq!(second[0].tag, Rating::KEEP_1);
    assert!(!second[0].looped);
    assert!(!second[0].missing);

    db.upsert_file(Path::new("one.wav"), 12, 6).unwrap();
    let third = db.list_files().unwrap();
    assert_eq!(third[0].tag, Rating::KEEP_1);
    assert!(!third[0].missing);

    let reopened = SourceDatabase::open(dir.path()).unwrap();
    let fourth = reopened.list_files().unwrap();
    assert_eq!(fourth[0].tag, Rating::KEEP_1);
    assert!(!fourth[0].looped);
    assert!(!fourth[0].missing);
}

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
fn loop_markers_default_and_persist() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("loop.wav"), 10, 5).unwrap();

    let first = db.list_files().unwrap();
    assert!(!first[0].looped);

    db.set_looped(Path::new("loop.wav"), true).unwrap();
    let second = db.list_files().unwrap();
    assert!(second[0].looped);

    db.upsert_file(Path::new("loop.wav"), 12, 6).unwrap();
    let third = db.list_files().unwrap();
    assert!(third[0].looped);

    let reopened = SourceDatabase::open(dir.path()).unwrap();
    let fourth = reopened.list_files().unwrap();
    assert!(fourth[0].looped);
}

#[test]
fn batch_tag_updates_coalesce_to_latest_value() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    db.set_tags_batch(&[
        (PathBuf::from("one.wav"), Rating::KEEP_1),
        (PathBuf::from("one.wav"), Rating::TRASH_1),
    ])
    .unwrap();

    let rows = db.list_files().unwrap();
    assert_eq!(rows[0].tag, Rating::TRASH_1);
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
fn missing_flag_round_trips() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    db.set_missing(Path::new("one.wav"), true).unwrap();
    let rows = db.list_files().unwrap();
    assert!(rows[0].missing);
    db.set_missing(Path::new("one.wav"), false).unwrap();
    let rows = db.list_files().unwrap();
    assert!(!rows[0].missing);
}

#[test]
fn list_and_count_only_show_supported_audio() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    db.upsert_file(Path::new("notes.txt"), 1, 1).unwrap();

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
    assert_eq!(db.count_files().unwrap(), 1);
    assert!(db.index_for_path(Path::new("notes.txt")).unwrap().is_none());
}

#[test]
fn wav_upsert_variants_preserve_hash_tag_and_missing_contracts() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(Path::new("one.wav"), 10, 5, "hash-a", Rating::KEEP_1, true)
        .unwrap();
    batch.commit().unwrap();

    let first = db.list_files().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].content_hash.as_deref(), Some("hash-a"));
    assert_eq!(first[0].tag, Rating::KEEP_1);
    assert!(first[0].missing);

    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_without_hash(Path::new("one.wav"), 12, 6)
        .unwrap();
    batch.commit().unwrap();

    let second = db.list_files().unwrap();
    assert_eq!(second[0].content_hash, None);
    assert_eq!(second[0].tag, Rating::KEEP_1);
    assert!(!second[0].missing);
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

#[test]
fn batch_bpm_lookup_returns_requested_sample_rows() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.connection
        .execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
                 VALUES (?1, 'h1', 1, 1, ?2)",
            params!["source::one.wav", 124.0f64],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
                 VALUES (?1, 'h2', 1, 1, NULL)",
            params!["source::two.wav"],
        )
        .unwrap();

    let lookup = db
        .bpms_for_sample_ids(&[
            String::from("source::one.wav"),
            String::from("source::two.wav"),
            String::from("source::missing.wav"),
        ])
        .unwrap();

    assert_eq!(lookup.get("source::one.wav"), Some(&Some(124.0)));
    assert_eq!(lookup.get("source::two.wav"), Some(&None));
    assert!(!lookup.contains_key("source::missing.wav"));
}
