use super::*;
use fixtures::{column_names, with_legacy_db};

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

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
    let columns = column_names(&db.connection, "pending_wav_renames");
    assert!(columns.iter().any(|column| column == "sound_type"));
    assert!(columns.iter().any(|column| column == "user_tag"));
    assert!(columns.iter().any(|column| column == "collections"));

    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].relative_path, std::path::Path::new("legacy.wav"));
    assert_eq!(pending[0].metadata.tag, Rating::KEEP_1);
    assert!(pending[0].metadata.looped);
    assert!(pending[0].metadata.locked);
    assert_eq!(pending[0].metadata.last_played_at, Some(42));
    assert_eq!(pending[0].metadata.sound_type, None);
    assert_eq!(pending[0].metadata.user_tag, None);
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

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
    let columns = column_names(&db.connection, "pending_wav_renames");
    assert!(columns.iter().any(|column| column == "sound_type"));
    assert!(columns.iter().any(|column| column == "user_tag"));
    assert!(columns.iter().any(|column| column == "collections"));

    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].relative_path,
        std::path::Path::new("current-stamped.wav")
    );
    assert_eq!(pending[0].metadata.sound_type, None);
    assert_eq!(pending[0].metadata.user_tag, None);
}

#[test]
fn legacy_single_collection_pending_row_restores_membership() {
    let dir = with_legacy_db(
        "CREATE TABLE wav_files (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            tag INTEGER NOT NULL DEFAULT 0,
            looped INTEGER NOT NULL DEFAULT 0,
            locked INTEGER NOT NULL DEFAULT 0,
            missing INTEGER NOT NULL DEFAULT 0,
            extension TEXT NOT NULL DEFAULT '',
            collection INTEGER
        );
        CREATE TABLE pending_wav_renames (
            path TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            content_hash TEXT,
            tag INTEGER NOT NULL,
            looped INTEGER NOT NULL,
            locked INTEGER NOT NULL,
            last_played_at INTEGER,
            collection INTEGER
        );
        INSERT INTO pending_wav_renames (
            path, file_size, modified_ns, content_hash, tag, looped, locked,
            last_played_at, collection
        ) VALUES (
            'legacy.wav', 10, 5, 'hash-a', 1, 0, 0, NULL, 3
        );",
    );

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
    let columns = column_names(&db.connection, "pending_wav_renames");
    assert!(columns.iter().any(|column| column == "collections"));
    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].metadata.collections,
        vec![SampleCollection::new(3).unwrap()]
    );

    db.upsert_file(std::path::Path::new("restored.wav"), 10, 5)
        .unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .restore_rename_metadata(std::path::Path::new("restored.wav"), &pending[0].metadata)
        .unwrap();
    batch.commit().unwrap();

    assert_eq!(
        db.collections_for_path(std::path::Path::new("restored.wav"))
            .unwrap(),
        vec![SampleCollection::new(3).unwrap()]
    );
}
