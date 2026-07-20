use super::*;
use fixtures::{column_names, with_legacy_db};

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

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
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

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
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
