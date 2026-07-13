use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use tempfile::tempdir;

use super::super::{DB_FILE_NAME, Rating, SampleCollection, SampleSoundType, SourceDatabase};

#[test]
fn list_files_page_orders_supported_audio_and_applies_offsets() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    for name in [
        "delta.wav",
        "alpha.wav",
        "._appledouble.wav",
        "drums/._nested.wav",
        "notes.txt",
        "charlie.wav",
        "bravo.wav",
    ] {
        db.upsert_file(Path::new(name), 10, 5).unwrap();
    }

    let page = db.list_files_page(2, 1).unwrap();
    let paths = page
        .into_iter()
        .map(|entry| entry.relative_path)
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![PathBuf::from("bravo.wav"), PathBuf::from("charlie.wav")]
    );
}

#[test]
fn audio_queries_ignore_appledouble_sidecars() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    for name in [
        "._alpha.wav",
        "alpha.wav",
        "drums/._kick.wav",
        "drums/kick.wav",
    ] {
        db.upsert_file(Path::new(name), 10, 5).unwrap();
    }

    let paths = db
        .list_files()
        .unwrap()
        .into_iter()
        .map(|entry| entry.relative_path)
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![PathBuf::from("alpha.wav"), PathBuf::from("drums/kick.wav")]
    );
    assert!(
        db.entry_for_path(Path::new("._alpha.wav"))
            .unwrap()
            .is_none()
    );
    assert_eq!(db.index_for_path(Path::new("alpha.wav")).unwrap(), Some(0));
    assert_eq!(
        db.index_for_path(Path::new("drums/kick.wav")).unwrap(),
        Some(1)
    );
}

#[test]
fn list_queries_skip_invalid_relative_paths() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("valid.wav"), 10, 5).unwrap();
    db.set_missing(Path::new("valid.wav"), true).unwrap();
    db.connection
        .execute(
            "INSERT INTO wav_files (path, file_size, modified_ns, content_hash, tag, looped, locked, missing, extension)
             VALUES (?1, 1, 1, NULL, 0, 0, 0, 1, 'wav')",
            params!["../escape.wav"],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO wav_files (path, file_size, modified_ns, content_hash, tag, looped, locked, missing, extension)
             VALUES (?1, 1, 1, NULL, 0, 0, 0, 1, 'wav')",
            params!["C:/absolute.wav"],
        )
        .unwrap();

    let listed = db.list_files().unwrap();
    let missing = db.list_missing_paths().unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].relative_path, PathBuf::from("valid.wav"));
    assert_eq!(missing, vec![PathBuf::from("valid.wav")]);
}

#[test]
fn bpm_queries_return_only_present_rows_and_preserve_null_values() {
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

    assert_eq!(
        db.bpm_for_sample_id("source::one.wav").unwrap(),
        Some(124.0)
    );
    assert_eq!(db.bpm_for_sample_id("source::two.wav").unwrap(), None);
    assert_eq!(db.bpm_for_sample_id("source::missing.wav").unwrap(), None);

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
    assert!(db.bpms_for_sample_ids(&[]).unwrap().is_empty());
}

#[test]
fn search_entry_metadata_matches_row_order_and_values() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("drums/snare.wav"), 10, 5).unwrap();
    db.upsert_file(Path::new("drums/kick.wav"), 10, 6).unwrap();
    db.set_tag(Path::new("drums/kick.wav"), Rating::KEEP_1)
        .unwrap();
    db.set_locked(Path::new("drums/snare.wav"), true).unwrap();
    db.set_last_played_at(Path::new("drums/snare.wav"), 42)
        .unwrap();

    let rows = db.list_search_entry_rows().unwrap();
    let metadata = db.list_search_entry_metadata().unwrap();

    assert_eq!(metadata.len(), rows.len());
    assert_eq!(
        metadata,
        rows.into_iter().map(|row| row.metadata).collect::<Vec<_>>()
    );
}

#[test]
fn sound_type_round_trips_for_path_queries() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("drums/kick.wav"), 10, 5).unwrap();
    db.set_sound_type(Path::new("drums/kick.wav"), Some(SampleSoundType::Kick))
        .unwrap();

    assert_eq!(
        db.sound_type_for_path(Path::new("drums/kick.wav")).unwrap(),
        Some(SampleSoundType::Kick)
    );
}

#[test]
fn collection_reads_use_canonical_membership_rows() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    db.upsert_file(Path::new("two.wav"), 10, 5).unwrap();
    db.connection
        .execute(
            "UPDATE wav_files SET collection = 1 WHERE path IN ('one.wav', 'two.wav')",
            [],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO wav_file_collections (path, collection) VALUES ('one.wav', 2)",
            [],
        )
        .unwrap();

    assert_eq!(
        db.collections_for_path(Path::new("one.wav")).unwrap(),
        vec![SampleCollection::new(2).unwrap()]
    );
    assert!(
        db.collections_for_path(Path::new("two.wav"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn legacy_read_only_collection_query_falls_back_to_wav_files_column() {
    let dir = tempdir().unwrap();
    let connection = Connection::open(dir.path().join(DB_FILE_NAME)).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE wav_files (
                path TEXT PRIMARY KEY,
                file_size INTEGER NOT NULL,
                modified_ns INTEGER NOT NULL,
                collection INTEGER
            );
            INSERT INTO wav_files (path, file_size, modified_ns, collection)
            VALUES ('one.wav', 10, 5, 3);",
        )
        .unwrap();
    drop(connection);

    let db = SourceDatabase::open_read_only(dir.path()).unwrap();
    let expected = SampleCollection::new(3).unwrap();
    assert_eq!(
        db.collections_for_path(Path::new("one.wav")).unwrap(),
        vec![expected]
    );
    assert_eq!(
        db.collection_for_path(Path::new("one.wav")).unwrap(),
        Some(expected)
    );
}

#[test]
fn legacy_read_only_database_without_last_curated_at_preserves_saved_metadata() {
    let dir = tempdir().unwrap();
    let connection = Connection::open(dir.path().join(DB_FILE_NAME)).unwrap();
    connection
        .execute_batch(
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
                sound_type TEXT,
                locked INTEGER NOT NULL DEFAULT 0,
                missing INTEGER NOT NULL DEFAULT 0,
                extension TEXT NOT NULL DEFAULT '',
                last_played_at INTEGER,
                user_tag TEXT,
                tag_named INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE source_tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                normalized_text TEXT NOT NULL UNIQUE,
                display_label TEXT NOT NULL
            );
            CREATE TABLE wav_file_tags (
                path TEXT NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (path, tag_id)
            ) WITHOUT ROWID;",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO wav_files (
                path, file_size, modified_ns, content_hash, tag, looped, sound_type,
                locked, missing, extension, last_played_at, user_tag, tag_named
             )
             VALUES (?1, 2048, 99, ?2, ?3, 1, ?4, 1, 0, 'wav', 123, ?5, 1)",
            params![
                "drums/kick.wav",
                "hash-kick",
                Rating::KEEP_1.as_i64(),
                SampleSoundType::Kick.token(),
                "808",
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO source_tags (normalized_text, display_label) VALUES (?1, ?2)",
            params!["warm", "Warm"],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO wav_file_tags (path, tag_id) VALUES (?1, ?2)",
            params!["drums/kick.wav", connection.last_insert_rowid()],
        )
        .unwrap();
    drop(connection);

    let db = SourceDatabase::open_read_only(dir.path()).unwrap();
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.relative_path, PathBuf::from("drums/kick.wav"));
    assert_eq!(row.content_hash.as_deref(), Some("hash-kick"));
    assert_eq!(row.tag, Rating::KEEP_1);
    assert!(row.looped);
    assert_eq!(row.sound_type, Some(SampleSoundType::Kick));
    assert!(row.locked);
    assert_eq!(row.last_played_at, Some(123));
    assert_eq!(row.last_curated_at, None);
    assert_eq!(row.user_tag.as_deref(), Some("808"));
    assert!(row.tag_named);
    assert_eq!(row.normal_tags, vec![String::from("Warm")]);

    let entry = db
        .entry_for_path(Path::new("drums/kick.wav"))
        .unwrap()
        .expect("legacy row should hydrate");
    assert_eq!(entry.tag, Rating::KEEP_1);

    let search_rows = db.list_search_entry_rows().unwrap();
    assert_eq!(search_rows.len(), 1);
    assert_eq!(search_rows[0].metadata.tag, Rating::KEEP_1);
    assert_eq!(search_rows[0].metadata.last_played_at, Some(123));
    assert_eq!(search_rows[0].metadata.last_curated_at, None);
    assert_eq!(
        search_rows[0].metadata.normal_tags,
        vec![String::from("Warm")]
    );
    assert!(search_rows[0].metadata.tag_named);

    let search_metadata = db.list_search_entry_metadata().unwrap();
    assert_eq!(search_metadata, vec![search_rows[0].metadata.clone()]);
    assert_eq!(
        db.last_curated_at_for_path(Path::new("drums/kick.wav"))
            .unwrap(),
        None
    );
}

#[test]
fn legacy_read_only_minimal_wav_files_schema_reads_with_defaults() {
    let dir = tempdir().unwrap();
    let connection = Connection::open(dir.path().join(DB_FILE_NAME)).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE wav_files (
                path TEXT PRIMARY KEY,
                file_size INTEGER NOT NULL,
                modified_ns INTEGER NOT NULL
            );
            INSERT INTO wav_files (path, file_size, modified_ns)
            VALUES
                ('nested/One.WAV', 2048, 99),
                ('nested/._sidecar.wav', 1, 1),
                ('notes.txt', 1, 1);",
        )
        .unwrap();
    drop(connection);

    let db = SourceDatabase::open_read_only(dir.path()).unwrap();
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.relative_path, PathBuf::from("nested/One.WAV"));
    assert_eq!(row.content_hash, None);
    assert_eq!(row.tag, Rating::NEUTRAL);
    assert!(!row.looped);
    assert_eq!(row.sound_type, None);
    assert!(!row.locked);
    assert!(!row.missing);
    assert_eq!(row.last_played_at, None);
    assert_eq!(row.last_curated_at, None);
    assert_eq!(row.user_tag, None);
    assert!(!row.tag_named);
    assert!(row.normal_tags.is_empty());
    assert_eq!(db.count_files().unwrap(), 1);
    assert_eq!(db.count_present_files().unwrap(), 1);
    assert_eq!(db.list_files_page(10, 0).unwrap().len(), 1);
    assert!(
        db.entry_for_path(Path::new("nested/One.WAV"))
            .unwrap()
            .is_some()
    );
    assert!(
        db.list_paths_with_content_hash("missing")
            .unwrap()
            .is_empty()
    );

    let search_rows = db.list_search_entry_rows().unwrap();
    assert_eq!(search_rows.len(), 1);
    assert_eq!(search_rows[0].metadata.tag, Rating::NEUTRAL);
    assert_eq!(search_rows[0].metadata.last_played_at, None);
    assert_eq!(search_rows[0].metadata.last_curated_at, None);
    assert!(search_rows[0].metadata.normal_tags.is_empty());
    assert!(!search_rows[0].metadata.tag_named);
    assert_eq!(
        db.list_search_entry_rows_for_paths(&[PathBuf::from("nested/One.WAV")])
            .unwrap()
            .len(),
        1
    );
    assert_eq!(db.list_search_entry_metadata().unwrap().len(), 1);
    assert!(
        db.collections_for_path(Path::new("nested/One.WAV"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.collection_for_path(Path::new("nested/One.WAV")).unwrap(),
        None
    );
}
