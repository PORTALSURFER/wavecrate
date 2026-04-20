use super::*;
use crate::sample_sources::scanner::scan_once;
use crate::sample_sources::{DB_FILE_NAME, SourceDatabase};
use hound::{SampleFormat, WavSpec, WavWriter};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, mpsc};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

const SOURCE_ID: &str = "source";

#[test]
fn progress_snapshot_rebuilds_when_scan_makes_pending_job_countable() {
    let _guard = contention_test_lock().lock().unwrap();
    let fixture = ContentionFixture::new();

    fixture.write_wav("later.wav");
    let sample = fixture.sample_metadata("later.wav");
    let sample_id = sample.sample_id.clone();
    let mut analysis_conn = open_source_db(&fixture.root).unwrap();
    upsert_samples(&mut analysis_conn, &[sample]).unwrap();
    enqueue_jobs(
        &mut analysis_conn,
        &[(sample_id.clone(), String::from("hash-later"))],
        DEFAULT_JOB_TYPE,
        10,
        SOURCE_ID,
    )
    .unwrap();

    let before_scan = current_progress(&analysis_conn, &fixture.root).unwrap();
    assert_eq!(before_scan.pending, 0);
    assert_eq!(before_scan.total(), 0);

    let scan_db = SourceDatabase::open_fast(&fixture.root).unwrap();
    let stats = scan_once(&scan_db).unwrap();

    assert_eq!(stats.added, 1);
    assert_eq!(stats.updated, 0);
    assert_eq!(stats.missing, 0);

    let rebuilt = current_progress(&analysis_conn, &fixture.root).unwrap();
    assert_eq!(rebuilt.pending, 1);
    assert_eq!(rebuilt.running, 0);
    assert_eq!(rebuilt.done, 0);
    assert_eq!(rebuilt.failed, 0);
    assert_eq!(rebuilt.samples_total, 1);
    assert_eq!(rebuilt.samples_pending_or_running, 1);
}

#[test]
fn mixed_scan_metadata_enqueue_and_completion_contention_stays_consistent() {
    let _guard = contention_test_lock().lock().unwrap();
    run_mixed_contention_round(1);
}

#[test]
#[ignore = "opt-in local soak for mixed source-db contention coverage"]
fn mixed_scan_metadata_enqueue_and_completion_contention_soak() {
    let _guard = contention_test_lock().lock().unwrap();
    for round in 0..8 {
        run_mixed_contention_round(round);
    }
}

fn run_mixed_contention_round(round: usize) {
    let fixture = ContentionFixture::new();
    fixture.write_wav("one.wav");
    let scan_db = SourceDatabase::open_fast(&fixture.root).unwrap();
    scan_once(&scan_db).unwrap();

    let one_sample = fixture.sample_metadata("one.wav");
    let one_sample_id = one_sample.sample_id.clone();
    let mut setup_conn = open_source_db(&fixture.root).unwrap();
    upsert_samples(&mut setup_conn, &[one_sample]).unwrap();
    enqueue_jobs(
        &mut setup_conn,
        &[(one_sample_id.clone(), format!("hash-one-{round}"))],
        DEFAULT_JOB_TYPE,
        20 + round as i64,
        SOURCE_ID,
    )
    .unwrap();
    let claimed = claim_next_job(&mut setup_conn, &fixture.root)
        .unwrap()
        .expect("expected claimed job");

    fixture.write_wav("two.wav");
    let two_sample = fixture.sample_metadata("two.wav");
    let two_sample_id = two_sample.sample_id.clone();

    let (release_tx, done_rx) = lock_db_until_released(&fixture.root);
    let root_for_metadata = fixture.root.clone();
    let metadata_handle = thread::spawn(move || {
        let db = SourceDatabase::open_fast(&root_for_metadata).unwrap();
        db.set_locked(Path::new("one.wav"), true).unwrap();
    });
    let root_for_scan = fixture.root.clone();
    let scan_handle = thread::spawn(move || {
        let db = SourceDatabase::open_fast(&root_for_scan).unwrap();
        scan_once(&db).unwrap()
    });
    let root_for_enqueue = fixture.root.clone();
    let enqueue_handle = thread::spawn(move || {
        let mut conn = open_source_db(&root_for_enqueue).unwrap();
        upsert_samples(&mut conn, &[two_sample]).unwrap();
        enqueue_jobs(
            &mut conn,
            &[(two_sample_id, format!("hash-two-{round}"))],
            DEFAULT_JOB_TYPE,
            40 + round as i64,
            SOURCE_ID,
        )
        .unwrap();
    });
    let root_for_done = fixture.root.clone();
    let done_handle = thread::spawn(move || {
        let conn = open_source_db(&root_for_done).unwrap();
        mark_done(&conn, claimed.id).unwrap();
    });

    release_tx.send(()).unwrap();
    done_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    metadata_handle.join().unwrap();
    let stats = scan_handle.join().unwrap();
    enqueue_handle.join().unwrap();
    done_handle.join().unwrap();

    assert_eq!(stats.added, 1);
    assert_eq!(stats.updated, 0);
    assert_eq!(stats.missing, 0);

    let verify_db = SourceDatabase::open_fast(&fixture.root).unwrap();
    assert_eq!(
        verify_db.locked_for_path(Path::new("one.wav")).unwrap(),
        Some(true)
    );
    assert!(
        verify_db
            .entry_for_path(Path::new("two.wav"))
            .unwrap()
            .is_some()
    );

    let verify_conn = open_source_db(&fixture.root).unwrap();
    let progress = current_progress(&verify_conn, &fixture.root).unwrap();
    assert_eq!(progress.pending, 1);
    assert_eq!(progress.running, 0);
    assert_eq!(progress.done, 1);
    assert_eq!(progress.failed, 0);
    assert_eq!(progress.samples_total, 2);
    assert_eq!(progress.samples_pending_or_running, 1);
}

fn contention_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_db_until_released(source_root: &Path) -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
    let (release_tx, release_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let (locked_tx, locked_rx) = mpsc::channel();
    let db_file = source_root.join(DB_FILE_NAME);
    thread::spawn(move || {
        let conn = Connection::open(db_file).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        conn.execute_batch("COMMIT").unwrap();
        done_tx.send(()).unwrap();
    });
    locked_rx.recv().unwrap();
    (release_tx, done_rx)
}

struct ContentionFixture {
    _temp: TempDir,
    root: PathBuf,
}

impl ContentionFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("source");
        std::fs::create_dir_all(&root).unwrap();
        Self { _temp: temp, root }
    }

    fn write_wav(&self, relative_path: &str) {
        let path = self.root.join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let spec = WavSpec {
            channels: 1,
            sample_rate: 8,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        for sample in [0.0_f32, 0.25, -0.25, 0.5] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn sample_metadata(&self, relative_path: &str) -> SampleMetadata {
        let absolute = self.root.join(relative_path);
        let metadata = std::fs::metadata(&absolute).unwrap();
        let modified_ns = metadata
            .modified()
            .unwrap()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;
        SampleMetadata {
            sample_id: build_sample_id(SOURCE_ID, Path::new(relative_path)),
            content_hash: format!("hash::{relative_path}"),
            size: metadata.len(),
            mtime_ns: modified_ns,
        }
    }
}
