use super::fixtures::{JobRow, TestDb};
use super::*;
use rusqlite::OptionalExtension;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn captured(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedBufferWriter(self.0.clone())
    }
}

struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn capture_debug_logs<F>(run: F) -> String
where
    F: FnOnce(),
{
    let buffer = SharedBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(buffer.clone())
        .finish();
    crate::logging::set_debug_logging_enabled_for_tests(true);
    wavecrate_library::diagnostics::set_debug_logging_enabled(true);
    tracing::subscriber::with_default(subscriber, run);
    wavecrate_library::diagnostics::set_debug_logging_enabled(false);
    crate::logging::set_debug_logging_enabled_for_tests(false);
    buffer.captured()
}

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
    let progress = current_progress(&db.conn, std::path::Path::new("/tmp")).unwrap();
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
    let progress = current_progress(&db.conn, std::path::Path::new("/tmp")).unwrap();
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
    let job = claim_next_job(&mut db.conn, Path::new("/tmp"))
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
fn empty_claim_path_stays_quiet_in_debug_logs() {
    let mut db = TestDb::new();

    let captured = capture_debug_logs(|| {
        let jobs = claim_next_jobs(&mut db.conn, Path::new("/tmp"), 1).unwrap();
        assert!(jobs.is_empty(), "expected no jobs to be claimable");
    });

    assert!(
        !captured.contains("analysis_claim_jobs"),
        "empty claim path should not emit claim transaction spam: {captured}"
    );
    assert!(
        !captured.contains("analysis.job.claim"),
        "empty claim path should not emit success claim actions: {captured}"
    );
    assert!(
        !captured.contains("source_db.open_total"),
        "empty claim path should not reopen source DBs during idle polling: {captured}"
    );
}

#[test]
fn retry_evidence_stays_visible_in_debug_logs() {
    let captured = capture_debug_logs(|| {
        telemetry::record_retry(
            "analysis_claim_jobs",
            Path::new("C:/tmp/source"),
            2,
            4,
            std::time::Duration::from_millis(75),
            "database is locked",
        );
    });

    assert!(
        captured.contains("Retrying source DB work after failure"),
        "retry path should stay visible in debug logs: {captured}"
    );
    assert!(
        captured.contains("action=\"retry\""),
        "retry path should preserve its retry classification: {captured}"
    );
    assert!(
        captured.contains("operation=\"analysis_claim_jobs\""),
        "retry path should preserve the operation name: {captured}"
    );
    assert!(
        captured.contains("error=\"database is locked\""),
        "retry path should preserve the lock error evidence: {captured}"
    );
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
