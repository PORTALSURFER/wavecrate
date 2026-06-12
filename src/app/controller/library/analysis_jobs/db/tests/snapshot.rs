use super::fixtures::{JobRow, TestDb};
use super::*;
use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;
use crate::sample_sources::{DB_FILE_NAME, db::META_WAV_PATHS_REVISION};
use rusqlite::params;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;

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

#[test]
fn ui_read_progress_recounts_stale_snapshot_without_requiring_repair_write() {
    let fixture = SnapshotFixture::new();
    seed_stale_analyze_snapshot(&fixture);
    let (release_tx, done_rx) = lock_db_until_released(&fixture.root);
    let ui_conn = open_source_db_ui_read(&fixture.root).unwrap();

    let progress = current_progress(&ui_conn, &fixture.root).unwrap();

    assert_eq!(
        progress,
        AnalysisProgress {
            pending: 0,
            running: 0,
            done: 1,
            failed: 1,
            samples_total: 2,
            samples_pending_or_running: 0,
        }
    );
    let snapshot_while_locked = read_snapshot_row(&fixture.root);
    assert_eq!(snapshot_while_locked, Some((0, 42, 11, 7)));
    assert_eq!(
        read_metadata_value(
            &fixture.root,
            "analysis_progress_snapshot_wav_paths_revision_v1"
        ),
        Some(String::from("137"))
    );

    release_tx.send(()).unwrap();
    done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
}

#[test]
fn writable_progress_reconciles_stale_snapshot_counts_and_revision() {
    let fixture = SnapshotFixture::new();
    seed_stale_analyze_snapshot(&fixture);
    let conn = open_source_db(&fixture.root).unwrap();

    let progress = current_progress(&conn, &fixture.root).unwrap();

    assert_eq!(
        progress,
        AnalysisProgress {
            pending: 0,
            running: 0,
            done: 1,
            failed: 1,
            samples_total: 2,
            samples_pending_or_running: 0,
        }
    );
    assert_eq!(read_snapshot_row(&fixture.root), Some((0, 0, 1, 1)));
    assert_eq!(
        read_metadata_value(
            &fixture.root,
            "analysis_progress_snapshot_wav_paths_revision_v1"
        ),
        Some(String::from("144"))
    );
}

struct SnapshotFixture {
    _temp: TempDir,
    root: std::path::PathBuf,
}

impl SnapshotFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("source");
        std::fs::create_dir_all(&root).unwrap();
        let _ = open_source_db(&root).unwrap();
        Self { _temp: temp, root }
    }
}

fn seed_stale_analyze_snapshot(fixture: &SnapshotFixture) {
    let conn = open_source_db(&fixture.root).unwrap();
    conn.execute(
        "INSERT INTO wav_files (path, file_size, modified_ns, tag, missing)
         VALUES (?1, 1, 1, 0, 0)",
        params!["a.wav"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO wav_files (path, file_size, modified_ns, tag, missing)
         VALUES (?1, 1, 1, 0, 0)",
        params!["b.wav"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO analysis_jobs (
            sample_id,
            source_id,
            relative_path,
            job_type,
            status,
            attempts,
            created_at,
            running_at,
            last_error
         ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, NULL, NULL)",
        params!["source::a.wav", "source", "a.wav", DEFAULT_JOB_TYPE, "done"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO analysis_jobs (
            sample_id,
            source_id,
            relative_path,
            job_type,
            status,
            attempts,
            created_at,
            running_at,
            last_error
         ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, NULL, NULL)",
        params![
            "source::b.wav",
            "source",
            "b.wav",
            DEFAULT_JOB_TYPE,
            "failed"
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO analysis_job_progress_snapshots (job_type, pending, running, done, failed)
         VALUES (?1, 0, 42, 11, 7)",
        params![DEFAULT_JOB_TYPE],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO metadata (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["analysis_progress_snapshot_wav_paths_revision_v1", "137"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO metadata (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![META_WAV_PATHS_REVISION, "144"],
    )
    .unwrap();
}

fn read_snapshot_row(root: &Path) -> Option<(i64, i64, i64, i64)> {
    let conn = open_source_db_ui_read(root).unwrap();
    conn.query_row(
        "SELECT pending, running, done, failed
         FROM analysis_job_progress_snapshots
         WHERE job_type = ?1",
        params![DEFAULT_JOB_TYPE],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .ok()
}

fn read_metadata_value(root: &Path, key: &str) -> Option<String> {
    let conn = open_source_db_ui_read(root).unwrap();
    conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

fn lock_db_until_released(root: &Path) -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
    let (release_tx, release_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let (locked_tx, locked_rx) = mpsc::channel();
    let db_file = root.join(DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        conn.execute_batch("COMMIT").unwrap();
        done_tx.send(()).unwrap();
    });
    locked_rx.recv().unwrap();
    (release_tx, done_rx)
}
