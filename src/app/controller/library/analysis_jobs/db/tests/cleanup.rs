use super::fixtures::{JobRow, SampleRow, TestDb};
use super::*;

#[test]
fn reset_running_to_pending_updates_rows() {
    let db = TestDb::new();
    db.insert_job(
        JobRow::new("s::a.wav", "x", "running")
            .with_attempts(1)
            .with_running_at(5),
    );
    let changed = reset_running_to_pending(&db.conn).unwrap();
    assert_eq!(changed, 1);
    let (status, running_at): (String, Option<i64>) = db
        .conn
        .query_row(
            "SELECT status, running_at FROM analysis_jobs WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "pending");
    assert_eq!(running_at, None);
}

#[test]
fn fail_stale_running_jobs_ignores_recent_claims() {
    let db = TestDb::new();
    db.insert_job(
        JobRow::new("s::old.wav", "x", "running")
            .with_attempts(1)
            .with_running_at(10),
    );
    db.insert_job(
        JobRow::new("s::fresh.wav", "x", "running")
            .with_attempts(1)
            .with_running_at(100),
    );
    let changed = fail_stale_running_jobs(&db.conn, 50).unwrap();
    assert_eq!(changed, 1);
    let status_old: String = db
        .conn
        .query_row(
            "SELECT status FROM analysis_jobs WHERE sample_id = 's::old.wav'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let status_fresh: String = db
        .conn
        .query_row(
            "SELECT status FROM analysis_jobs WHERE sample_id = 's::fresh.wav'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status_old, "failed");
    assert_eq!(status_fresh, "running");
}

#[test]
fn fail_stale_running_jobs_marks_failed() {
    let db = TestDb::new();
    db.insert_job(
        JobRow::new("s::a.wav", "x", "running")
            .with_attempts(1)
            .with_running_at(10),
    );
    let changed = fail_stale_running_jobs(&db.conn, 20).unwrap();
    assert_eq!(changed, 1);
    let (status, last_error): (String, Option<String>) = db
        .conn
        .query_row(
            "SELECT status, last_error FROM analysis_jobs WHERE sample_id = 's::a.wav'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "failed");
    assert!(last_error.unwrap_or_default().contains("Timed out"));
}

#[test]
fn prune_jobs_for_missing_sources_removes_orphans() {
    let db = TestDb::new();
    db.insert_wav_file("a.wav");
    db.insert_job(JobRow::new("s::a.wav", ANALYZE_SAMPLE_JOB_TYPE, "pending"));
    db.insert_job(JobRow::new(
        "missing::b.wav",
        ANALYZE_SAMPLE_JOB_TYPE,
        "pending",
    ));
    let removed = prune_jobs_for_missing_sources(&db.conn).unwrap();
    assert_eq!(removed, 1);
    let remaining: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM analysis_jobs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(remaining, 1);
}

#[test]
fn purge_orphaned_samples_removes_rows_from_all_tables() {
    let mut db = TestDb::new();
    db.insert_wav_file("a.wav");
    for sample_id in ["s::a.wav", "missing::b.wav"] {
        db.insert_sample(SampleRow::new(sample_id, "h"));
        db.insert_job(JobRow::new(sample_id, ANALYZE_SAMPLE_JOB_TYPE, "pending"));
        db.insert_orphaned_artifacts(sample_id);
    }
    let removed = purge_orphaned_samples(&mut db.conn).unwrap();
    assert_eq!(removed, 5);
    for table in [
        "samples",
        "analysis_jobs",
        "analysis_features",
        "features",
        "embeddings",
    ] {
        let count: i64 = db
            .conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE sample_id = 'missing::b.wav'"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}
