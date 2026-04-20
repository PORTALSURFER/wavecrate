use super::fixtures::{JobRow, TestDb};
use super::*;
use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;

fn full_recount_analyze(conn: &rusqlite::Connection) -> AnalysisProgress {
    let mut progress = AnalysisProgress::default();
    let mut stmt = conn
        .prepare(
            "SELECT aj.status, COUNT(*)
             FROM analysis_jobs aj
             JOIN wav_files wf
               ON wf.path = aj.relative_path
             WHERE aj.job_type = ?1
             GROUP BY aj.status",
        )
        .unwrap();
    let mut rows = stmt.query([DEFAULT_JOB_TYPE]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let status: String = row.get(0).unwrap();
        let count = row.get::<_, i64>(1).unwrap().max(0) as usize;
        match status.as_str() {
            "pending" => progress.pending = count,
            "running" => progress.running = count,
            "done" => progress.done = count,
            "failed" => progress.failed = count,
            _ => {}
        }
    }
    progress.samples_total = progress.total();
    progress.samples_pending_or_running = progress.pending + progress.running;
    progress
}

#[test]
fn current_progress_bootstraps_snapshot_from_existing_rows() {
    let db = TestDb::new();
    db.insert_wav_file("a.wav");
    db.insert_wav_file("b.wav");
    db.insert_job(JobRow::new("s::a.wav", DEFAULT_JOB_TYPE, "pending").with_source("s", "a.wav"));
    db.insert_job(JobRow::new("s::b.wav", DEFAULT_JOB_TYPE, "done").with_source("s", "b.wav"));

    let progress = current_progress(&db.conn, std::path::Path::new("/tmp")).unwrap();

    assert_eq!(
        progress,
        AnalysisProgress {
            pending: 1,
            running: 0,
            done: 1,
            failed: 0,
            samples_total: 2,
            samples_pending_or_running: 1,
        }
    );
}

#[test]
fn snapshot_tracks_many_small_status_transitions() {
    let mut db = TestDb::new();
    for idx in 0..8 {
        db.insert_wav_file(&format!("clip-{idx}.wav"));
    }
    let jobs: Vec<(String, String)> = (0..8)
        .map(|idx| (format!("s::clip-{idx}.wav"), format!("hash-{idx}")))
        .collect();
    enqueue_jobs(&mut db.conn, &jobs, DEFAULT_JOB_TYPE, 10, "s").unwrap();

    for idx in 0..8 {
        let claimed = claim_next_job(&mut db.conn, std::path::Path::new("/tmp"))
            .unwrap()
            .expect("claimed job");
        if idx % 3 == 0 {
            mark_failed_with_reason(&db.conn, claimed.id, "boom").unwrap();
        } else {
            mark_done(&db.conn, claimed.id).unwrap();
        }
        let snapshot = current_progress(&db.conn, std::path::Path::new("/tmp")).unwrap();
        let recount = full_recount_analyze(&db.conn);
        assert_eq!(snapshot, recount);
    }

    let final_progress = current_progress(&db.conn, std::path::Path::new("/tmp")).unwrap();
    assert_eq!(final_progress.pending, 0);
    assert_eq!(final_progress.running, 0);
    assert_eq!(final_progress.done, 5);
    assert_eq!(final_progress.failed, 3);
    assert_eq!(final_progress.samples_total, 8);
    assert_eq!(final_progress.samples_pending_or_running, 0);
}
