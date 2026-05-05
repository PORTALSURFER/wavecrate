use super::fixtures::{SampleRow, TestDb};
use super::*;

#[test]
fn sample_bpm_round_trips() {
    let db = TestDb::new();
    db.insert_sample(SampleRow::new("s::a.wav", "hash"));
    update_sample_bpm(&db.conn, "s::a.wav", Some(128.0)).unwrap();
    let bpm = sample_bpm(&db.conn, "s::a.wav").unwrap();
    assert_eq!(bpm, Some(128.0));
    update_sample_bpm(&db.conn, "s::a.wav", None).unwrap();
    let cleared = sample_bpm(&db.conn, "s::a.wav").unwrap();
    assert_eq!(cleared, None);
}

#[test]
fn update_sample_bpms_updates_multiple_rows() {
    let mut db = TestDb::new();
    db.insert_sample(SampleRow::new("s::a.wav", "hash-a"));
    db.insert_sample(SampleRow::new("s::b.wav", "hash-b"));
    let sample_ids = vec!["s::a.wav".to_string(), "s::b.wav".to_string()];
    let updated = update_sample_bpms(&mut db.conn, &sample_ids, Some(96.0)).unwrap();
    assert_eq!(updated, 2);
    let bpm_a = sample_bpm(&db.conn, "s::a.wav").unwrap();
    let bpm_b = sample_bpm(&db.conn, "s::b.wav").unwrap();
    assert_eq!(bpm_a, Some(96.0));
    assert_eq!(bpm_b, Some(96.0));
}

#[test]
fn upsert_samples_preserves_bpm_on_hash_change() {
    let mut db = TestDb::new();
    db.insert_sample(SampleRow::new("s::a.wav", "hash-a"));
    update_sample_bpm(&db.conn, "s::a.wav", Some(124.0)).unwrap();
    let samples = vec![SampleMetadata {
        sample_id: "s::a.wav".to_string(),
        content_hash: "hash-b".to_string(),
        size: 2,
        mtime_ns: 2,
    }];
    upsert_samples(&mut db.conn, &samples).unwrap();
    let bpm = sample_bpm(&db.conn, "s::a.wav").unwrap();
    assert_eq!(bpm, Some(124.0));
}

#[test]
fn upsert_samples_preserves_long_mark_on_fast_hash_upgrade() {
    let mut db = TestDb::new();
    db.insert_sample(
        SampleRow::new("s::a.wav", "fast-10-5")
            .with_file_state(10, 5)
            .with_duration(12.0)
            .with_long_mark(1),
    );
    let samples = vec![SampleMetadata {
        sample_id: "s::a.wav".to_string(),
        content_hash: "full-hash".to_string(),
        size: 10,
        mtime_ns: 5,
    }];
    upsert_samples(&mut db.conn, &samples).unwrap();
    let (duration, mark): (Option<f64>, Option<i64>) = db
        .conn
        .query_row(
            "SELECT duration_seconds, long_sample_mark FROM samples WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(duration, Some(12.0));
    assert_eq!(mark, Some(1));
}

#[test]
fn upsert_samples_clears_long_mark_on_edit() {
    let mut db = TestDb::new();
    db.insert_sample(
        SampleRow::new("s::a.wav", "fast-10-5")
            .with_file_state(10, 5)
            .with_duration(12.0)
            .with_long_mark(1),
    );
    let samples = vec![SampleMetadata {
        sample_id: "s::a.wav".to_string(),
        content_hash: "full-hash".to_string(),
        size: 11,
        mtime_ns: 6,
    }];
    upsert_samples(&mut db.conn, &samples).unwrap();
    let (duration, mark): (Option<f64>, Option<i64>) = db
        .conn
        .query_row(
            "SELECT duration_seconds, long_sample_mark FROM samples WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(duration, None);
    assert_eq!(mark, None);
}
