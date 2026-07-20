use super::*;
use fixtures::{column_names, with_legacy_db};

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

    let db = SourceDatabase::open_for_test_fixture_source_write(dir.path()).unwrap();
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
