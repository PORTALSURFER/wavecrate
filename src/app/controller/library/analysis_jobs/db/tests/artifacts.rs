use super::fixtures::{SampleRow, TestDb};
use super::*;

#[test]
fn update_analysis_metadata_updates_matching_hash() {
    let db = TestDb::new();
    db.insert_sample(SampleRow::new("s::a.wav", "h1").with_file_state(10, 5));
    update_analysis_metadata(
        &db.conn,
        AnalysisMetadataUpdate {
            sample_id: "s::a.wav",
            content_hash: Some("h1"),
            duration_seconds: 1.25,
            sr_used: crate::analysis::audio::ANALYSIS_SAMPLE_RATE,
            analysis_version: "analysis_v1_test",
        },
    )
    .unwrap();
    let (duration, sr, version): (Option<f64>, Option<i64>, Option<String>) = db
        .conn
        .query_row(
            "SELECT duration_seconds, sr_used, analysis_version FROM samples WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(duration, Some(1.25));
    assert_eq!(
        sr,
        Some(crate::analysis::audio::ANALYSIS_SAMPLE_RATE as i64)
    );
    assert_eq!(version.as_deref(), Some("analysis_v1_test"));
}

#[test]
fn update_sample_duration_preserves_analysis_version() {
    let db = TestDb::new();
    db.insert_sample(
        SampleRow::new("s::a.wav", "h1")
            .with_file_state(10, 5)
            .with_analysis_version("analysis_v1_test"),
    );
    update_sample_duration(
        &db.conn,
        "s::a.wav",
        2.5,
        crate::analysis::audio::ANALYSIS_SAMPLE_RATE,
    )
    .unwrap();
    let (duration, version): (Option<f64>, Option<String>) = db
        .conn
        .query_row(
            "SELECT duration_seconds, analysis_version FROM samples WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(duration, Some(2.5));
    assert_eq!(version.as_deref(), Some("analysis_v1_test"));
}

#[test]
fn update_sample_duration_updates_when_hash_differs() {
    let db = TestDb::new();
    db.insert_sample(SampleRow::new("s::a.wav", "old-hash").with_file_state(10, 5));
    let updated = update_sample_duration(
        &db.conn,
        "s::a.wav",
        3.0,
        crate::analysis::audio::ANALYSIS_SAMPLE_RATE,
    )
    .unwrap();
    assert!(updated);
    let (duration, hash): (Option<f64>, Option<String>) = db
        .conn
        .query_row(
            "SELECT duration_seconds, content_hash FROM samples WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(duration, Some(3.0));
    assert_eq!(hash.as_deref(), Some("old-hash"));
}

#[test]
fn update_sample_duration_creates_row_on_load() {
    let mut db = TestDb::new();
    let samples = vec![SampleMetadata {
        sample_id: "s::a.wav".to_string(),
        content_hash: "fast-10-5".to_string(),
        size: 10,
        mtime_ns: 5,
    }];
    upsert_samples(&mut db.conn, &samples).unwrap();
    let updated = update_sample_duration(
        &db.conn,
        "s::a.wav",
        4.0,
        crate::analysis::audio::ANALYSIS_SAMPLE_RATE,
    )
    .unwrap();
    assert!(updated);
    let duration: Option<f64> = db
        .conn
        .query_row(
            "SELECT duration_seconds FROM samples WHERE sample_id = 's::a.wav'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(duration, Some(4.0));
}

#[test]
fn sample_ids_missing_duration_finds_nulls() {
    let db = TestDb::new();
    db.insert_sample(SampleRow::new("s::missing.wav", "h1").with_file_state(10, 5));
    db.insert_sample(
        SampleRow::new("s::ok.wav", "h2")
            .with_file_state(10, 5)
            .with_duration(1.0),
    );
    let missing = sample_ids_missing_duration(
        &db.conn,
        &["s::missing.wav".to_string(), "s::ok.wav".to_string()],
    )
    .unwrap();
    assert!(missing.contains("s::missing.wav"));
    assert!(!missing.contains("s::ok.wav"));
}

#[test]
fn upsert_analysis_features_overwrites_existing() {
    let db = TestDb::new();
    upsert_analysis_features(&db.conn, "s::a.wav", b"one", 1, 100).unwrap();
    upsert_analysis_features(&db.conn, "s::a.wav", b"two", 1, 200).unwrap();
    let (version, blob, computed_at): (i64, Vec<u8>, i64) = db
        .conn
        .query_row(
            "SELECT feat_version, vec_blob, computed_at FROM features WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(version, 1);
    assert_eq!(blob, b"two");
    assert_eq!(computed_at, 200);
}
