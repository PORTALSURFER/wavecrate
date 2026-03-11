use super::fixtures::{JobRow, TestDb};
use super::*;
use rusqlite::OptionalExtension;

#[test]
fn enqueue_jobs_dedupes_by_sample_and_type() {
    let mut db = TestDb::new();
    db.insert_wav_file("a.wav");
    let jobs = vec![
        ("s::a.wav".to_string(), "h1".to_string()),
        ("s::a.wav".to_string(), "h1".to_string()),
    ];
    let inserted = enqueue_jobs(&mut db.conn, &jobs, DEFAULT_JOB_TYPE, 123, "s").unwrap();
    assert_eq!(inserted, 2);
    let progress = current_progress(&db.conn).unwrap();
    assert_eq!(progress.pending, 1);
    assert_eq!(progress.total(), 1);
}

#[test]
fn progress_uses_relative_path_over_sample_id() {
    let db = TestDb::new();
    db.insert_wav_file("a.wav");
    db.insert_job(
        JobRow::new("s::wrong.wav", DEFAULT_JOB_TYPE, "pending").with_source("s", "a.wav"),
    );
    let progress = current_progress(&db.conn).unwrap();
    assert_eq!(progress.total(), 1);
    assert_eq!(progress.pending, 1);
}

#[test]
fn ann_index_dirty_marker_round_trips() {
    let db = TestDb::new();
    mark_ann_index_dirty(&db.conn, "failed").unwrap();
    let value: String = db
        .conn
        .query_row(
            "SELECT value FROM metadata WHERE key = 'ann_index_dirty_v1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(value.contains("failed"));
    clear_ann_index_dirty(&db.conn).unwrap();
    let cleared: Option<String> = db
        .conn
        .query_row(
            "SELECT value FROM metadata WHERE key = 'ann_index_dirty_v1'",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert!(cleared.is_none());
}

#[test]
fn enqueue_rebuild_job_dedupes_pending() {
    let db = TestDb::new();
    let inserted = enqueue_rebuild_ann_index_job(&db.conn, "s", 10).unwrap();
    assert_eq!(inserted, 1);
    let second = enqueue_rebuild_ann_index_job(&db.conn, "s", 11).unwrap();
    assert_eq!(second, 0);
    let count: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE job_type = ?1",
            rusqlite::params![REBUILD_INDEX_JOB_TYPE],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn claim_next_job_marks_running_and_increments_attempts() {
    let mut db = TestDb::new();
    let jobs = vec![("s::a.wav".to_string(), "h1".to_string())];
    enqueue_jobs(&mut db.conn, &jobs, DEFAULT_JOB_TYPE, 123, "s").unwrap();
    let job = claim_next_job(&mut db.conn, std::path::Path::new("/tmp"))
        .unwrap()
        .expect("job claimed");
    assert_eq!(job.sample_id, "s::a.wav");
    assert_eq!(job.content_hash.as_deref(), Some("h1"));
    assert_eq!(job.job_type, DEFAULT_JOB_TYPE);
    let (status, attempts): (String, i64) = db
        .conn
        .query_row(
            "SELECT status, attempts FROM analysis_jobs WHERE id = ?1",
            rusqlite::params![job.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "running");
    assert_eq!(attempts, 1);
}

#[test]
fn mark_done_clears_error_and_updates_status() {
    let db = TestDb::new();
    db.insert_job(
        JobRow::new("s::a.wav", "x", "running")
            .with_attempts(1)
            .with_last_error("oops"),
    );
    let job_id: i64 = db
        .conn
        .query_row("SELECT id FROM analysis_jobs", [], |row| row.get(0))
        .unwrap();
    mark_done(&db.conn, job_id).unwrap();
    let (status, last_error): (String, Option<String>) = db
        .conn
        .query_row(
            "SELECT status, last_error FROM analysis_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "done");
    assert_eq!(last_error, None);
}

#[test]
fn mark_failed_updates_status_and_error() {
    let db = TestDb::new();
    db.insert_job(JobRow::new("s::a.wav", "x", "running").with_attempts(1));
    let job_id: i64 = db
        .conn
        .query_row("SELECT id FROM analysis_jobs", [], |row| row.get(0))
        .unwrap();
    mark_failed_with_reason(&db.conn, job_id, "boom").unwrap();
    let (status, last_error): (String, Option<String>) = db
        .conn
        .query_row(
            "SELECT status, last_error FROM analysis_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(last_error.as_deref(), Some("boom"));
}
